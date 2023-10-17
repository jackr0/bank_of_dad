use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::http;
use bank_of_dad::model::amount::Amount;
use bank_of_dad::model::transaction::Transaction;
use bank_of_dad::{db::Db, router};
use chrono::DateTime;
use futures::StreamExt;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper::Client;
use hyper::Request;
use hyper::StatusCode;
use log::info;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

async fn post(
    client: &Client<HttpConnector>,
    uri: String,
    request_body: String,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
        .body(Body::from(request_body))
        .unwrap();

    let response = client.request(request).await.unwrap();

    let status_code = response.status();

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();

    (status_code, body)
}

async fn open_web_socket(
    addr: SocketAddr,
    child_name: &str,
) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let (socket, _response) =
        tokio_tungstenite::connect_async(format!("ws://{addr}/child/{child_name}/notifications"))
            .await
            .unwrap();

    socket
}

async fn get_next_transaction_frame_from_socket(
    socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Transaction {
    let msg = match timeout(Duration::from_secs(1), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
    {
        tungstenite::Message::Text(msg) => msg,
        other => panic!("unexpected websocket message {other:?}"),
    };
    serde_json::from_str::<Transaction>(&msg).unwrap()
}

async fn post_give_to_child(
    client: &Client<HttpConnector>,
    addr: SocketAddr,
    child_name: &str,
    msg: &str,
) {
    let (status_code, _body) = post(
        client,
        format!("http://{addr}/child/{child_name}/give"),
        String::from(msg),
    )
    .await;
    assert_eq!(status_code, StatusCode::OK);
}

#[tokio::test]
async fn websocket_e2e_test() {
    tracing_subscriber::fmt().with_thread_ids(true).init();

    let db = Db::new();
    let app = router(db);
    let client = Client::new();

    let server = axum::Server::bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .serve(app.into_make_service());
    let addr = server.local_addr();
    info!("websocket_e2e_test running on port {}", addr);
    tokio::spawn(server);

    let mut socket_a1 = open_web_socket(addr, "a").await;
    let mut socket_a2 = open_web_socket(addr, "a").await;
    let mut socket_b1 = open_web_socket(addr, "b").await;

    //
    // Give a payment and validate it's recieved by both websockets listening for child A
    //
    post_give_to_child(
        &client,
        addr,
        "a",
        r#"{"amount":5.99,"purpose":"pocket money 1"}"#,
    )
    .await;

    let expected_notification = Transaction::new(
        1,
        DateTime::UNIX_EPOCH,
        String::from("a"),
        Amount::from_pence(599),
        String::from("pocket money 1"),
    );

    let trx_a1 = get_next_transaction_frame_from_socket(&mut socket_a1).await;
    assert_eq!(
        Transaction {
            timestamp: DateTime::UNIX_EPOCH,
            ..trx_a1
        },
        expected_notification
    );

    let trx_a2 = get_next_transaction_frame_from_socket(&mut socket_a2).await;
    assert_eq!(
        Transaction {
            timestamp: DateTime::UNIX_EPOCH,
            ..trx_a2
        },
        expected_notification
    );

    //
    // Give a payment to B and validate it is next message on the web socket.
    // This asserts that the transaction for child A was not sent to B.
    //
    post_give_to_child(
        &client,
        addr,
        "b",
        r#"{"amount":10,"purpose":"pocket money 2"}"#,
    )
    .await;

    let expected_notification = Transaction::new(
        2,
        DateTime::UNIX_EPOCH,
        String::from("b"),
        Amount::from_pence(1000),
        String::from("pocket money 2"),
    );

    let trx_b = get_next_transaction_frame_from_socket(&mut socket_b1).await;
    assert_eq!(
        Transaction {
            timestamp: DateTime::UNIX_EPOCH,
            ..trx_b
        },
        expected_notification
    );
}
