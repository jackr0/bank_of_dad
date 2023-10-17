use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    Router,
};
use bank_of_dad::{
    db::Db,
    handlers::child::ChildAccountResponse,
    model::{amount::Amount, transaction::Transaction},
    router,
};
use chrono::DateTime;
use serde_json::{json, Value};
use tower::{Service, ServiceExt};

async fn get(app: &mut Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.ready().await.unwrap().call(request).await.unwrap();

    let status_code = response.status();

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();

    (status_code, body)
}

async fn post(app: &mut Router, uri: &str, request_body: String) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
        .body(Body::from(request_body.clone()))
        .unwrap();
    let response = app.ready().await.unwrap().call(request).await.unwrap();

    let status_code = response.status();

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();

    (status_code, body)
}

fn normalize_transaction(transaction: Transaction) -> Transaction {
    Transaction {
        timestamp: DateTime::UNIX_EPOCH,
        ..transaction
    }
}

fn to_normalized_transaction(json: Value) -> Transaction {
    normalize_transaction(serde_json::from_value::<Transaction>(json).unwrap())
}

fn normalize_timestamps_in_child_account_response(
    child_account_response: ChildAccountResponse,
) -> ChildAccountResponse {
    let transactions = child_account_response
        .transactions
        .iter()
        .map(|t| normalize_transaction(t.clone()))
        .collect::<Vec<Transaction>>();

    ChildAccountResponse {
        transactions: transactions,
        ..child_account_response
    }
}

fn to_normalised_child_account_response(json: Value) -> ChildAccountResponse {
    normalize_timestamps_in_child_account_response(
        serde_json::from_value::<ChildAccountResponse>(json).unwrap(),
    )
}

fn transaction(id: u8, child_name: &str, amount: i64, purpose: &str) -> Transaction {
    Transaction {
        id: id,
        timestamp: DateTime::UNIX_EPOCH,
        child_name: child_name.to_string(),
        amount: Amount::from_pence(amount),
        purpose: purpose.to_string(),
    }
}

fn child_account_response(
    child_name: &str,
    balance: i64,
    transactions: Vec<Transaction>,
) -> ChildAccountResponse {
    ChildAccountResponse {
        child_name: child_name.to_string(),
        balance: Amount::from_pence(balance),
        transactions: transactions,
    }
}

#[tokio::test]
async fn e2e_test() {
    tracing_subscriber::fmt().with_thread_ids(true).init();

    let db = Db::new();
    let mut app = router(db);

    //
    // Assert Child a and Child b have no transactions
    //
    let (status_code, body) = get(&mut app, "/child/a").await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalised_child_account_response(body),
        child_account_response("a", 0, vec![])
    );

    let (status_code, body) = get(&mut app, "/child/b").await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalised_child_account_response(body),
        child_account_response("b", 0, vec![])
    );

    //
    // Try and spend some money, expect rejection
    //
    let (status_code, body) = post(
        &mut app,
        "/child/a/spend",
        String::from(r#"{"amount":5.99,"purpose":"negative test"}"#),
    )
    .await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        json!({"reason": "Transaction will take account a negative"})
    );

    //
    // Give money to child A, then validate this transaction exists
    //
    let (status_code, body) = post(
        &mut app,
        "/child/a/give",
        String::from(r#"{"amount":5.99,"purpose":"pocket money 1"}"#),
    )
    .await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalized_transaction(body),
        transaction(1, "a", 599, "pocket money 1")
    );

    let (status_code, body) = get(&mut app, "/child/a").await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalised_child_account_response(body),
        child_account_response("a", 599, vec![transaction(1, "a", 599, "pocket money 1")])
    );

    //
    // Check rejection if too try to spend too much
    //
    let (status_code, body) = post(
        &mut app,
        "/child/a/spend",
        String::from(r#"{"amount":10.00,"purpose":"negative test 2"}"#),
    )
    .await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        json!({"reason": "Transaction will take account a negative"})
    );

    //
    // Child A can spend within account balance, then validate this transaction exists
    //
    let (status_code, body) = post(
        &mut app,
        "/child/a/spend",
        String::from(r#"{"amount":3.00,"purpose":"permitted spend"}"#),
    )
    .await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalized_transaction(body),
        transaction(2, "a", -300, "permitted spend")
    );

    let (status_code, body) = get(&mut app, "/child/a").await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalised_child_account_response(body),
        child_account_response(
            "a",
            299,
            vec![
                transaction(1, "a", 599, "pocket money 1"),
                transaction(2, "a", -300, "permitted spend")
            ]
        )
    );

    //
    // Check no change for child B
    //
    let (status_code, body) = get(&mut app, "/child/b").await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(
        to_normalised_child_account_response(body),
        child_account_response("b", 0, vec![])
    );
}
