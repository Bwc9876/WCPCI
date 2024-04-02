use std::collections::HashMap;
use std::sync::Arc;

use log::error;
use rocket_db_pools::Pool;
use tokio::sync::Mutex;

use crate::contests::{Contest, Participant};
use crate::db::DbPool;
use crate::problems::{JudgeRun, ProblemCompletion};

use super::job::{Job, JobRequest};

use super::languages::RunConfig;
use super::{JobState, JobStateReceiver};

type UserId = i64;

type RunHandle = Arc<Mutex<Option<(i64, JobStateReceiver)>>>;

pub type JobStartedMessage = (UserId, i64, JobStateReceiver);
pub type JobStartedReceiver = tokio::sync::broadcast::Receiver<JobStartedMessage>;
pub type JobStartedSender = tokio::sync::broadcast::Sender<JobStartedMessage>;

pub type ProblemUpdatedMessage = ();
pub type ProblemUpdatedReceiver = tokio::sync::watch::Receiver<ProblemUpdatedMessage>;
pub type ProblemUpdatedSender = tokio::sync::watch::Sender<ProblemUpdatedMessage>;

pub type ShutdownReceiver = tokio::sync::watch::Receiver<bool>;

pub struct RunManager {
    config: RunConfig,
    id_counter: u64,
    jobs: HashMap<UserId, RunHandle>,
    db_pool: DbPool,
    job_started_channel: (JobStartedSender, JobStartedReceiver),
    problem_updated_channels: HashMap<i64, ProblemUpdatedSender>,
    shutdown_rx: ShutdownReceiver,
}

impl RunManager {
    pub fn new(config: RunConfig, pool: DbPool, shutdown_rx: ShutdownReceiver) -> Self {
        let (tx, rx) = tokio::sync::broadcast::channel(10);
        Self {
            config,
            id_counter: 1,
            jobs: HashMap::with_capacity(10),
            db_pool: pool,
            job_started_channel: (tx, rx),
            problem_updated_channels: HashMap::with_capacity(5),
            shutdown_rx,
        }
    }

    pub fn subscribe(&self) -> JobStartedReceiver {
        self.job_started_channel.0.subscribe()
    }

    pub fn subscribe_shutdown(&self) -> ShutdownReceiver {
        self.shutdown_rx.clone()
    }

    async fn start_job(&mut self, request: JobRequest) -> Result<(), String> {
        let id = self.id_counter;
        self.id_counter += 1;

        let user_id = request.user_id;
        let participant_id = request.participant_id;
        let problem_id = request.problem_id;
        let contest_id = request.contest_id;

        let language_config = self
            .config
            .languages
            .get(&request.language)
            .ok_or_else(|| format!("Language {} not supported by runner", request.language))?;

        let (job, state_rx) = Job::new(id, request, self.shutdown_rx.clone(), language_config)
            .await
            .map_err(|e| {
                error!("Couldn't create job: {:?}", e);
                "Judge Error".to_string()
            })?;

        let handle = Arc::new(Mutex::new(Some((problem_id, state_rx.clone()))));

        self.jobs.insert(user_id, handle.clone());

        let pool = self.db_pool.clone();

        tokio::spawn(async move {
            let (state, ran_at) = job.run().await;
            handle.lock().await.take();
            drop(handle);
            if !matches!(state, JobState::Judging { .. }) {
                return;
            }
            if let Some(participant_id) = participant_id {
                let judge_run =
                    JudgeRun::from_job_state(problem_id, participant_id, &state, ran_at);
                match pool.get().await {
                    Ok(mut conn) => {
                        if let Some(contest) = Contest::get(&mut conn, contest_id).await {
                            if !contest.is_running() {
                                return;
                            }

                            if let Some(participant) =
                                Participant::get(&mut conn, contest_id, user_id).await
                            {
                                if participant.is_judge {
                                    return;
                                }

                                let success = judge_run.success();
                                if let Err(why) = judge_run.write_to_db(&mut conn).await {
                                    error!("Couldn't write judge run to db: {:?}", why);
                                }

                                let mut completion =
                                    ProblemCompletion::get_for_problem_and_participant(
                                        &mut conn, problem_id, user_id,
                                    )
                                    .await
                                    .unwrap_or_else(|| {
                                        ProblemCompletion::temp(
                                            user_id,
                                            problem_id,
                                            Some(ran_at).filter(|_| success),
                                        )
                                    });

                                if success {
                                    completion.completed_at = Some(ran_at);
                                } else if state.last_error().1 && completion.completed_at.is_none()
                                {
                                    completion.number_wrong += 1;
                                }

                                if let Err(why) = completion.upsert(&mut conn).await {
                                    error!("Couldn't write problem completion to db: {:?}", why);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Couldn't get db connection: {:?}", e);
                    }
                }
            }
        });

        self.job_started_channel
            .0
            .send((user_id, problem_id, state_rx))
            .ok();

        Ok(())
    }

    pub fn get_handle_for_problem(&mut self, problem_id: i64) -> ProblemUpdatedReceiver {
        if let Some(handle) = self.problem_updated_channels.get(&problem_id) {
            handle.subscribe()
        } else {
            let (tx, rx) = tokio::sync::watch::channel(());
            self.problem_updated_channels.insert(problem_id, tx);
            rx
        }
    }

    pub async fn update_problem(&mut self, problem_id: i64) {
        if let Some(handle) = self.problem_updated_channels.remove(&problem_id) {
            handle.send(()).ok();
        }
    }

    pub async fn get_handle(&self, user_id: UserId, problem_id: i64) -> Option<JobStateReceiver> {
        if let Some(handle) = self.jobs.get(&user_id) {
            let handle = handle.lock().await;
            handle
                .as_ref()
                .filter(|(id, _)| *id == problem_id)
                .map(|(_, rx)| rx.clone())
        } else {
            None
        }
    }

    pub async fn request_job(&mut self, request: JobRequest) -> Result<(), String> {
        if let Some(handle) = self.jobs.get(&request.user_id) {
            let handle = handle.lock().await;
            if handle.is_some() {
                Err("User already has a job running".to_string())
            } else {
                drop(handle);
                self.start_job(request).await
            }
        } else {
            self.start_job(request).await
        }
    }
}
