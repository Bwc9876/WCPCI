use std::sync::Arc;

use rocket::{fairing::AdHoc, routes};
use rocket_db_pools::Database as R_Database;
use tokio::sync::Mutex;

use crate::db::Database;

use self::manager::RunManager;

mod job;
mod manager;
mod runner;
mod ws;

pub type JobStateMessage = job::JobState;

pub type JobStateSender = tokio::sync::watch::Sender<JobStateMessage>;
pub type JobStateReceiver = tokio::sync::watch::Receiver<JobStateMessage>;

pub type ManagerHandle = Arc<Mutex<RunManager>>;

pub use job::JobState;

pub fn stage() -> AdHoc {
    let (tx, rx) = tokio::sync::watch::channel(false);

    AdHoc::try_on_ignite("Runner App", |rocket| async {
        let pool = match Database::fetch(&rocket) {
            Some(pool) => pool.0.clone(), // clone the wrapped pool
            None => return Err(rocket),
        };

        let shutdown_fairing = AdHoc::on_shutdown("Shutdown Runners / Sockets", |_| {
            Box::pin(async move {
                tx.send(true).ok();
            })
        });

        let manager = manager::RunManager::new(pool, rx);
        Ok(rocket
            .attach(shutdown_fairing)
            .manage::<ManagerHandle>(Arc::new(Mutex::new(manager)))
            .mount("/run", routes![ws::ws_channel]))
    })
}
