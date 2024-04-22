use log::error;
use rocket::{
    futures::{SinkExt, StreamExt},
    get,
    http::Status,
    State,
};
use rocket_ws::{stream::DuplexStream, WebSocket};
use tokio::select;

use crate::{contests::Contest, db::DbConnection};

use super::{
    manager::{LeaderboardUpdateMessage, LeaderboardUpdateReceiver, ShutdownReceiver},
    LeaderboardManagerHandle,
};

enum LoopRes {
    NoOp,
    Break,
    Msg(LeaderboardUpdateMessage),
}

async fn websocket_loop(
    mut stream: DuplexStream,
    mut rx: LeaderboardUpdateReceiver,
    mut shutdown_rx: ShutdownReceiver,
) {
    loop {
        let res = select! {
            client_message = stream.next() => {
                if let Some(client_message) = client_message {
                    match client_message {
                        Ok(rocket_ws::Message::Close(_)) => LoopRes::Break,
                        _ => LoopRes::NoOp
                    }
                } else {
                    LoopRes::Break
                }
            }
            leaderboard_update = rx.recv() => {
                match leaderboard_update {
                    Ok(msg) => LoopRes::Msg(msg),
                    Err(e) => {
                        error!("Error receiving leaderboard update: {:?}", e);
                        LoopRes::NoOp
                    }
                }
            }
            Ok(()) = shutdown_rx.changed() => {
                LoopRes::Break
            }
        };

        match res {
            LoopRes::Break => break,
            LoopRes::Msg(msg) => {
                let json_string = serde_json::to_string(&msg).unwrap();
                let res = stream.send(rocket_ws::Message::Text(json_string)).await;
                if let Err(e) = res {
                    error!("Error sending message: {:?}", e);
                }
            }
            _ => {}
        }
    }
}

#[get("/contests/<contest_id>/leaderboard/ws")]
pub async fn leaderboard_ws(
    ws: WebSocket,
    mut db: DbConnection,
    contest_id: i64,
    manager: &State<LeaderboardManagerHandle>,
) -> Result<rocket_ws::Channel<'static>, Status> {
    let contest = Contest::get(&mut db, contest_id)
        .await
        .ok_or(Status::NotFound)?;
    let mut manager = manager.lock().await;
    let rx = manager.subscribe_leaderboard(&mut db, &contest).await;
    let shutdown_rx = manager.subscribe_shutdown();
    Ok(ws.channel(move |stream| {
        Box::pin(async move {
            websocket_loop(stream, rx, shutdown_rx).await;
            Ok(())
        })
    }))
}
