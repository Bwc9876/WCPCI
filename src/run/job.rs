use log::{error, info, warn};
use rocket::time::OffsetDateTime;

use crate::{problems::TestCase, run::runner::CaseError};

use super::{manager::ShutdownReceiver, runner::Runner, JobStateReceiver, JobStateSender};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(tag = "status", content = "content", rename_all = "camelCase")]
pub enum CaseStatus {
    #[default]
    Pending,
    Running,
    Passed(Option<String>),
    NotRun,
    Failed(String),
}

impl CaseStatus {
    pub fn to_name(&self) -> &str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Passed(_) => "Passed",
            Self::NotRun => "NotRun",
            Self::Failed(_) => "Failed",
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JobState {
    Judging {
        cases: Vec<CaseStatus>,
        complete: bool,
    },
    Testing {
        status: CaseStatus,
    },
}

impl JobState {
    pub fn new_judging(cases: usize) -> Self {
        Self::Judging {
            cases: vec![CaseStatus::Pending; cases],
            complete: false,
        }
    }

    pub fn new_testing() -> Self {
        Self::Testing {
            status: CaseStatus::Pending,
        }
    }

    pub fn last_error(&self) -> (usize, Option<String>) {
        match self {
            Self::Judging { cases, .. } => cases
                .iter()
                .enumerate()
                .find_map(|(i, c)| {
                    if let CaseStatus::Failed(e) = c {
                        Some((i, Some(e.clone())))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| (self.len(), None)),
            Self::Testing { status } => {
                if let CaseStatus::Failed(e) = status {
                    (0, Some(e.clone()))
                } else {
                    (0, None)
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Judging { cases, .. } => cases.len(),
            Self::Testing { .. } => 1,
        }
    }

    pub fn complete(&self) -> bool {
        match self {
            Self::Judging { complete, .. } => *complete,
            Self::Testing { status } => matches!(
                status,
                CaseStatus::Passed(_) | CaseStatus::Failed(_) | CaseStatus::NotRun
            ),
        }
    }

    pub fn start_first(&mut self) {
        match self {
            Self::Judging { cases, .. } => {
                cases[0] = CaseStatus::Running;
            }
            Self::Testing { status } => {
                *status = CaseStatus::Running;
            }
        }
    }

    pub fn complete_case(&mut self, idx: usize, status: CaseStatus) {
        match self {
            Self::Judging { cases, complete } => {
                if idx == cases.len() - 1 {
                    *complete = true;
                } else if matches!(&status, CaseStatus::Failed(_)) {
                    cases
                        .iter_mut()
                        .skip(idx + 1)
                        .for_each(|c| *c = CaseStatus::NotRun);
                    *complete = true;
                } else {
                    cases[idx + 1] = CaseStatus::Running;
                }
                cases[idx] = status;
            }
            Self::Testing { status: my_status } => {
                *my_status = status;
            }
        }
    }
}

pub enum JobOperation {
    Judging(Vec<TestCase>),
    Testing(String),
}

pub struct JobRequest {
    pub user_id: i64,
    pub problem_id: i64,
    pub program: String,
    pub cpu_time: i64,
    pub op: JobOperation,
}

pub struct Job {
    pub id: u64,
    user_id: i64,
    runner: Option<Runner>,
    op: JobOperation,
    pub state: JobState,
    state_tx: JobStateSender,
    started_at: OffsetDateTime,
    shutdown_rx: ShutdownReceiver,
}

impl Job {
    pub async fn new(
        id: u64,
        request: JobRequest,
        shutdown_rx: ShutdownReceiver,
    ) -> (Self, JobStateReceiver) {
        let mut state = match request.op {
            JobOperation::Judging(ref cases) => JobState::new_judging(cases.len()),
            JobOperation::Testing(_) => JobState::new_testing(),
        };
        let res = Runner::new(&request.program, request.cpu_time).await;
        let runner = match res {
            Ok(runner) => {
                info!("Job {} Runner created", id);
                Some(runner)
            }
            Err(e) => {
                error!("Job {} Couldn't create runner: {:?}", id, e);
                state.complete_case(
                    0,
                    CaseError::Judge("Couldn't create runner".to_string()).into(),
                );
                None
            }
        };
        let (state_tx, state_rx) = tokio::sync::watch::channel(state.clone());
        (
            Self {
                id,
                runner,
                state: state.clone(),
                state_tx,
                user_id: request.user_id,
                op: request.op,
                started_at: OffsetDateTime::now_utc(),
                shutdown_rx,
            },
            state_rx,
        )
    }

    pub async fn run(mut self) -> (JobState, OffsetDateTime) {
        let runner = match self.runner {
            Some(ref runner) => runner,
            None => {
                warn!("Job {} not running due to missing runner", self.id);
                return (self.state, self.started_at);
            }
        };
        info!(
            "Job {} Starting, requested by user {}",
            self.id, self.user_id
        );
        self.state.start_first();
        self.publish_state();
        match &self.op {
            JobOperation::Judging(cases) => {
                for (i, case) in cases.iter().enumerate() {
                    info!("Job {} Running Case {}", self.id, i + 1);
                    let status = match runner.run_case(case).await {
                        Ok(_) => CaseStatus::Passed(None),
                        Err(e) => match &e {
                            CaseError::Judge(ref why) => {
                                error!(
                                    "Job {} Case {} had a judging error: {:?}",
                                    self.id,
                                    i + 1,
                                    why
                                );
                                e.into()
                            }
                            _ => e.into(),
                        },
                    };
                    info!(
                        "Job {} Case {} finished with status {:?}",
                        self.id,
                        i + 1,
                        status.to_name()
                    );
                    self.state.complete_case(i, status);
                    self.publish_state();
                    if self.state.complete() {
                        break;
                    }
                    if self.shutdown_rx.has_changed().unwrap_or(false) {
                        info!("Job {} Received Shutdown Signal, Cancelling", self.id);
                        return (self.state, self.started_at);
                    }
                }
            }
            JobOperation::Testing(input) => {
                info!("Job {} Running Test", self.id);
                let status = match runner.run_cmd(input).await {
                    Ok(out) => CaseStatus::Passed(Some(out)),
                    Err(e) => match &e {
                        CaseError::Judge(ref why) => {
                            error!("Job {} Test had a judging error: {:?}", self.id, why);
                            e.into()
                        }
                        CaseError::Runtime(ref msg) => {
                            CaseStatus::Failed(format!("Runtime Error: {}", msg))
                        }
                        _ => e.into(),
                    },
                };
                info!(
                    "Job {} Test finished with status {:?}",
                    self.id,
                    status.to_name()
                );
                self.state.complete_case(0, status);
                self.publish_state();
            }
        }

        info!("Job {} Finished", self.id);
        (self.state, self.started_at)
    }

    pub fn publish_state(&self) {
        let res = self.state_tx.send(self.state.clone());
        if let Err(why) = res {
            error!("Job {} Couldn't send job state: {:?}", self.id, why);
        }
    }
}
