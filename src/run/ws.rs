use log::error;
use rocket::{
    futures::{SinkExt, StreamExt},
    get,
    http::Status,
    State,
};
use rocket_ws::{stream::DuplexStream, WebSocket};
use serde::Deserialize;
use tokio::select;

use crate::{
    auth::users::{Admin, User},
    contests::{Contest, Participant},
    db::DbConnection,
    problems::{Problem, TestCase},
    run::job::{JobOperation, JobRequest},
};

use super::{JobState, JobStateReceiver, ManagerHandle};

#[derive(Responder)]
pub enum WsHttpResponse {
    Accept(rocket_ws::Channel<'static>),
    Reject(Status),
}

// Keep in sync with TypeScript type
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WebSocketRequest {
    Judge {
        program: String,
        language: String,
    },
    Test {
        program: String,
        language: String,
        input: String,
    },
}

impl WebSocketRequest {
    pub fn program(&self) -> &str {
        match self {
            Self::Judge { program, .. } => program,
            Self::Test { program, .. } => program,
        }
    }

    pub fn language(&self) -> &str {
        match self {
            Self::Judge { language, .. } => language,
            Self::Test { language, .. } => language,
        }
    }
}

// Keep in sync with TypeScript type
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WebSocketMessage {
    StateUpdate { state: JobState },
    RunStarted,
    RunDenied { reason: String },
    Invalid { error: String },
}

enum LoopRes {
    Msg(WebSocketMessage),
    ChangeJobRx(JobStateReceiver),
    JobStart(JobRequest),
    Pong(Vec<u8>),
    Break,
    NoOp,
}

async fn websocket_loop(
    mut stream: DuplexStream,
    manager: ManagerHandle,
    problem: Problem,
    test_cases: Vec<TestCase>,
    user_id: i64,
) {
    let _manager = manager.lock().await;
    let mut started_rx = _manager.subscribe();
    let mut shutdown_rx = _manager.subscribe_shutdown();
    let state_rx = _manager.get_handle(user_id, problem.id).await;
    drop(_manager);
    // Fake receiver to start the loop, will be replaced by the real one
    let (_, fake_rx) = tokio::sync::watch::channel(JobState::new_judging(0));

    let mut state_msg = None;

    let mut state_rx: JobStateReceiver = if let Some(rx) = state_rx {
        let r = rx.borrow();
        let msg = serde_json::to_string(&WebSocketMessage::StateUpdate { state: r.clone() })
            .map_err(|e| e.to_string())
            .unwrap();
        state_msg = Some(msg);
        drop(r);
        rx
    } else {
        fake_rx
    };

    if let Some(msg) = state_msg {
        let res = stream.send(rocket_ws::Message::Text(msg)).await;
        if let Err(e) = res {
            error!("Error sending message: {:?}", e);
        }
    }

    loop {
        let res = select! {
            Ok((user_id_incoming, problem_id, rx)) = started_rx.recv() => {
                if user_id_incoming == user_id && problem_id == problem.id {
                    LoopRes::ChangeJobRx(rx)
                } else {
                    LoopRes::NoOp
                }
            }
            client_message = stream.next() => {
                if let Some(client_message) = client_message {
                    if let Ok(client_message) = client_message {
                        match client_message {
                            rocket_ws::Message::Text(raw) => {
                                if let Ok(request) = serde_json::from_str::<WebSocketRequest>(&raw) {
                                    let op = match &request {
                                        WebSocketRequest::Judge { .. } => JobOperation::Judging(test_cases.clone()),
                                        WebSocketRequest::Test { input, .. } => JobOperation::Testing(input.to_string())
                                    };
                                    let job_to_start = JobRequest {
                                        user_id,
                                        problem_id: problem.id,
                                        contest_id: problem.contest_id,
                                        program: request.program().to_string(),
                                        language: request.language().to_string(),
                                        cpu_time: problem.cpu_time,
                                        op
                                    };
                                    LoopRes::JobStart(job_to_start)
                                } else {
                                    LoopRes::Msg(WebSocketMessage::Invalid { error: "Invalid request".to_string() })
                                }
                            },
                            rocket_ws::Message::Ping(e) => {
                                LoopRes::Pong(e)
                            },
                            rocket_ws::Message::Close(_) => {
                                LoopRes::Break
                            },
                            _ => {
                                LoopRes::NoOp
                            }
                        }
                    } else {
                        LoopRes::NoOp
                    }
                } else {
                    LoopRes::Break
                }
            }
            Ok(()) = state_rx.changed() => {
                let state = state_rx.borrow();
                LoopRes::Msg(WebSocketMessage::StateUpdate { state: state.clone() })
            }
            Ok(()) = shutdown_rx.changed() => {
                LoopRes::Break
            }
        };

        let mut state_rx_changed_msg = None;

        match res {
            LoopRes::Msg(msg) => {
                let msg = serde_json::to_string(&msg)
                    .map_err(|e| e.to_string())
                    .unwrap();
                let res = stream.send(rocket_ws::Message::Text(msg)).await;
                if let Err(e) = res {
                    error!("Error sending message: {:?}", e);
                }
            }
            LoopRes::JobStart(job) => {
                let mut manager = manager.lock().await;
                let msg = match manager.request_job(job).await {
                    Ok(_) => WebSocketMessage::RunStarted,
                    Err(why) => WebSocketMessage::RunDenied { reason: why },
                };
                drop(manager);
                let msg = serde_json::to_string(&msg)
                    .map_err(|e| e.to_string())
                    .unwrap();
                let res = stream.send(rocket_ws::Message::Text(msg)).await;
                if let Err(e) = res {
                    error!("Error sending message: {:?}", e);
                }
            }
            LoopRes::Pong(e) => {
                let res = stream.send(rocket_ws::Message::Pong(e)).await;
                if let Err(e) = res {
                    error!("Error sending pong: {:?}", e);
                }
            }
            LoopRes::Break => {
                break;
            }
            LoopRes::ChangeJobRx(rx) => {
                state_rx = rx;
                let state = state_rx.borrow();
                let msg = serde_json::to_string(&WebSocketMessage::StateUpdate {
                    state: state.clone(),
                })
                .map_err(|e| e.to_string())
                .unwrap();
                state_rx_changed_msg = Some(msg);
            }
            _ => {}
        }

        if let Some(msg) = state_rx_changed_msg {
            let res = stream.send(rocket_ws::Message::Text(msg)).await;
            if let Err(e) = res {
                error!("Error sending message: {:?}", e);
            }
        }
    }
}

#[get("/ws/<contest_id>/<problem_id>")]
pub async fn ws_channel(
    ws: WebSocket,
    contest_id: i64,
    problem_id: i64,
    user: &User,
    admin: Option<&Admin>,
    manager: &State<ManagerHandle>,
    mut db: DbConnection,
) -> WsHttpResponse {
    if let Some(problem) = Problem::by_id(&mut db, contest_id, problem_id).await {
        if let Some(contest) = Contest::get(&mut db, contest_id).await {
            let participant = Participant::get(&mut db, contest_id, user.id).await;
            let is_judge = participant.as_ref().map(|p| p.is_judge).unwrap_or(false);
            let is_admin = admin.is_some();
            if !is_admin
                && !is_judge
                && (!contest.has_started() || contest.is_running() && participant.is_none())
            {
                return WsHttpResponse::Reject(Status::Forbidden);
            }

            let handle = (*manager).clone();
            // TODO: Handle test cases being updated while the user is connected
            let cases = TestCase::get_for_problem(&mut db, problem_id)
                .await
                .unwrap_or_default();
            if !cases.is_empty() {
                let user_id = user.id;
                WsHttpResponse::Accept(ws.channel(move |stream| {
                    Box::pin(async move {
                        websocket_loop(stream, handle, problem, cases, user_id).await;
                        Ok(())
                    })
                }))
            } else {
                WsHttpResponse::Reject(Status::NotFound)
            }
        } else {
            WsHttpResponse::Reject(Status::NotFound)
        }
    } else {
        WsHttpResponse::Reject(Status::NotFound)
    }
}
