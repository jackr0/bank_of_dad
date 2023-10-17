use std::sync::Arc;

use log::info;
use tokio::sync::RwLock;

use crate::{
    db::Db,
    model::websocket_msg::{ActiveWebsocket, WebSocketMsg},
};

pub struct AppState {
    db: Arc<Db>,
    open_websockets: RwLock<Vec<ActiveWebsocket>>,
}

impl AppState {
    pub fn new(db: Db) -> AppState {
        AppState {
            db: Arc::new(db),
            open_websockets: RwLock::new(Vec::new()),
        }
    }

    pub fn get_db(&self) -> Arc<Db> {
        return self.db.clone();
    }

    pub async fn register_open_websocket(&self, websocket: ActiveWebsocket) {
        let mut websockets: tokio::sync::RwLockWriteGuard<'_, Vec<ActiveWebsocket>> =
            self.open_websockets.write().await;
        let child_name = websocket.get_child_name();
        websockets.push(websocket);
        info!(
            "registered websocket for {}. {} websockets registered",
            child_name,
            websockets.len()
        )
    }

    pub async fn deregister_open_websocket(&self, id: String) {
        let mut websockets: tokio::sync::RwLockWriteGuard<'_, Vec<ActiveWebsocket>> =
            self.open_websockets.write().await;
        websockets.retain(|ws| ws.get_id().ne(&id));
        info!("{} websockets registered", websockets.len())
    }

    pub async fn queue_messages_to_active_websockets_for_child(
        &self,
        child_name: String,
        msg: WebSocketMsg,
    ) {
        let websockets = self.open_websockets.read().await;

        let eligible_websockets: Vec<&ActiveWebsocket> = websockets
            .iter()
            .filter(|ws| ws.get_child_name() == child_name)
            .collect();

        for eligible_websocket in eligible_websockets {
            info!(
                "sending message to websocket {}",
                eligible_websocket.get_id()
            );
            let _ = eligible_websocket.send_message(msg.clone()).await;
        }

        return;
    }
}
