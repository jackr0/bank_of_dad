use tokio::sync::mpsc::error::SendError;

use super::transaction::Transaction;

#[derive(Debug, Clone)]
pub enum WebSocketMsg {
    CloseSocket(),
    Transaction(Transaction),
}

#[derive(Debug, Clone)]
pub struct ActiveWebsocket {
    id: String,
    child_name: String,
    send_channel: tokio::sync::mpsc::Sender<WebSocketMsg>,
}

impl ActiveWebsocket {
    pub fn new(
        id: String,
        child_name: String,
        send_channel: tokio::sync::mpsc::Sender<WebSocketMsg>,
    ) -> ActiveWebsocket {
        ActiveWebsocket {
            id: id,
            child_name: child_name,
            send_channel: send_channel,
        }
    }

    pub fn get_id(&self) -> String {
        return self.id.clone();
    }

    pub fn get_child_name(&self) -> String {
        return self.child_name.clone();
    }

    pub async fn send_message(&self, msg: WebSocketMsg) -> Result<(), SendError<WebSocketMsg>> {
        return self.send_channel.send(msg).await;
    }

    pub fn get_log_prefix(&self) -> String {
        return format!("[{}::{}]", self.id, self.child_name);
    }
}
