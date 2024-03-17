use std::collections::HashMap;
use std::sync::Arc;

use log::{error, info};
use rocket_db_pools::Pool;
use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::problems::JudgeRun;

use super::job::{Job, JobRequest};

use super::{JobState, JobStateReceiver};

type UserId = i64;

type RunHandle = Arc<Mutex<Option<(i64, JobStateReceiver)>>>;

pub type JobStartedMessage = (UserId, i64, JobStateReceiver);
pub type JobStartedReceiver = tokio::sync::broadcast::Receiver<JobStartedMessage>;
pub type JobStartedSender = tokio::sync::broadcast::Sender<JobStartedMessage>;

pub type ShutdownReceiver = tokio::sync::watch::Receiver<bool>;

pub struct RunManager {
    id_counter: u64,
    jobs: HashMap<UserId, RunHandle>,
    db_pool: DbPool,
    job_started_channel: (JobStartedSender, JobStartedReceiver),
    shutdown_rx: ShutdownReceiver,
}

impl RunManager {
    pub fn new(pool: DbPool, shutdown_rx: ShutdownReceiver) -> Self {
        let (tx, rx) = tokio::sync::broadcast::channel(10);
        Self {
            id_counter: 1,
            jobs: HashMap::with_capacity(10),
            db_pool: pool,
            job_started_channel: (tx, rx),
            shutdown_rx,
        }
    }

    pub fn subscribe(&self) -> JobStartedReceiver {
        self.job_started_channel.0.subscribe()
    }

    pub fn subscribe_shutdown(&self) -> ShutdownReceiver {
        self.shutdown_rx.clone()
    }

    async fn start_job(&mut self, request: JobRequest) {
        let id = self.id_counter;
        self.id_counter += 1;

        let user_id = request.user_id;
        let problem_id = request.problem_id;

        let (job, state_rx) = Job::new(id, request, self.shutdown_rx.clone()).await;

        let handle = Arc::new(Mutex::new(Some((problem_id, state_rx.clone()))));

        self.jobs.insert(user_id, handle.clone());

        let pool = self.db_pool.clone();

        tokio::spawn(async move {
            let (state, ran_at) = job.run().await;
            handle.lock().await.take();
            drop(handle);
            if matches!(state, JobState::Judging { .. }) {
                let judge_run = JudgeRun::from_job_state(problem_id, user_id, state, ran_at);
                match pool.get().await {
                    Ok(mut conn) => match judge_run.write_to_db(&mut conn).await {
                        Ok(_) => {
                            info!("Judge run written to db");
                        }
                        Err(e) => {
                            error!("Couldn't write judge run to db: {:?}", e);
                        }
                    },
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

    pub async fn request_job(&mut self, request: JobRequest) -> bool {
        if let Some(handle) = self.jobs.get(&request.user_id) {
            let handle = handle.lock().await;
            if handle.is_some() {
                false
            } else {
                drop(handle);
                self.start_job(request).await;
                true
            }
        } else {
            self.start_job(request).await;
            true
        }
    }
}
