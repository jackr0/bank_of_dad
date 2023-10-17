use std::{borrow::Cow, error::Error, sync::Arc};

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    Extension,
};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{info, warn};

use crate::{
    appstate::AppState, middleware::request_tracing::RequestTraceData,
    model::websocket_msg::ActiveWebsocket, model::websocket_msg::WebSocketMsg,
};

pub async fn accept_websocket(
    ws: WebSocketUpgrade,
    Path(child_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(request_trace_data): Extension<RequestTraceData>,
) -> impl IntoResponse {
    let request_id = request_trace_data.get_id();
    info!("[{request_id}] websocket accepted for child {child_name}.");

    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            app_state,
            request_trace_data,
            child_name.to_string(),
        )
    })
}

async fn handle_socket(
    socket: WebSocket,
    app_state: Arc<AppState>,
    request_trace_data: RequestTraceData,
    child_name: String,
) {
    let request_id = request_trace_data.get_id();
    let (ws_sender, ws_receiver) = socket.split();
    let (ch_sender, ch_receiver) = tokio::sync::mpsc::channel::<WebSocketMsg>(32);

    let websocket_details =
        ActiveWebsocket::new(request_id.clone(), child_name.clone(), ch_sender.clone());
    app_state
        .register_open_websocket(websocket_details.clone())
        .await;

    let ws_details_for_outgoing_task = websocket_details.clone();
    let ws_outgoing_task = tokio::spawn(async move {
        websocket_outgoing(ws_details_for_outgoing_task, ch_receiver, ws_sender).await
    });

    let ws_incoming_task =
        tokio::spawn(async move { websocket_incoming(websocket_details, ws_receiver).await });

    let _ = tokio::join!(ws_outgoing_task, ws_incoming_task);
    app_state.deregister_open_websocket(request_id).await;
}

async fn websocket_outgoing(
    active_websocket: ActiveWebsocket,
    mut rcv_channel: tokio::sync::mpsc::Receiver<WebSocketMsg>,
    mut ws_sender: SplitSink<WebSocket, Message>,
) -> () {
    let log_prefix = active_websocket.get_log_prefix();
    info!("{} Starting outgoing websocket", log_prefix);

    loop {
        let msg = rcv_channel.recv().await;

        match msg {
            Some(WebSocketMsg::CloseSocket()) => {
                info!("{} close socket recieved", log_prefix);
                let ws_send_res: Result<(), axum::Error> = ws_sender
                    .send(Message::Close(Some(CloseFrame {
                        code: axum::extract::ws::close_code::NORMAL,
                        reason: Cow::from("Goodbye"),
                    })))
                    .await;
                log_on_error(&log_prefix, "Close", "ws_sender", ws_send_res);

                break;
            }
            Some(WebSocketMsg::Transaction(t)) => {
                info!("{} notification recieved: {:?}", log_prefix, t);
                let ws_send_res = ws_sender
                    .send(Message::Text(serde_json::to_string(&t).unwrap()))
                    .await;
                log_on_error(&log_prefix, "Text", "ws_sender", ws_send_res);
            }
            None => {
                info!("{} none recieved, all senders likely dropped", log_prefix);

                let ws_send_res = ws_sender
                    .send(Message::Close(Some(CloseFrame {
                        code: axum::extract::ws::close_code::NORMAL,
                        reason: Cow::from("Goodbye"),
                    })))
                    .await;
                log_on_error(&log_prefix, "Close", "ws_sender", ws_send_res);

                break;
            }
        }
    }

    info!("{} websocket_outgoing exiting", log_prefix);
}

async fn websocket_incoming(
    active_websocket: ActiveWebsocket,
    mut websocket_reciever: SplitStream<WebSocket>,
) -> () {
    let log_prefix = active_websocket.get_log_prefix();
    loop {
        let frame = websocket_reciever.next().await;
        {
            match frame {
                Some(Ok(Message::Text(msg))) => {
                    info!("{} ok {}", log_prefix, msg);
                }
                Some(Ok(Message::Close(_))) => {
                    info!("{} ok close", log_prefix);
                    let ws_send_res = active_websocket
                        .send_message(WebSocketMsg::CloseSocket())
                        .await;
                    log_on_error(&log_prefix, "Close/Close", "active_websocket", ws_send_res);
                    break;
                }
                Some(Ok(_)) => {
                    info!("{} ok unhandled", log_prefix);
                }
                Some(Err(_)) => {
                    info!("{} ok error", log_prefix);
                    let ws_send_res = active_websocket
                        .send_message(WebSocketMsg::CloseSocket())
                        .await;
                    log_on_error(&log_prefix, "Close/Err", "active_websocket", ws_send_res);
                    break;
                }
                None => {
                    info!("{} None", log_prefix);
                    let ws_send_res = active_websocket
                        .send_message(WebSocketMsg::CloseSocket())
                        .await;
                    log_on_error(&log_prefix, "Close/None", "active_websocket", ws_send_res);
                    break;
                }
            }
        }
    }
    info!("{} websocket_incoming exiting", log_prefix);
}

fn log_on_error<E>(log_prefix: &str, msg_type: &str, channel_name: &str, result: Result<(), E>)
where
    E: Error,
{
    if let Err(e) = result {
        warn!(
            "{} Failed to add {} message to {} channel: {}",
            log_prefix, msg_type, channel_name, e
        );
    }
}
