use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Extension, Json,
};

use chrono::Utc;
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    appstate::AppState,
    middleware::request_tracing::RequestTraceData,
    model::{amount::Amount, error::ApiError, transaction::Transaction},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct GiveMoney {
    pub amount: Amount,
    pub purpose: String,
}

#[derive(Debug, PartialEq)]
enum TransactionType {
    Give,
    Spend,
}

fn validate_request_body(give_money: &GiveMoney) -> Result<(), ApiError> {
    if !give_money.amount.is_positive_nonzero() {
        return Err(ApiError::InputFailedValidation(String::from(
            "Amount must be at least 0.01",
        )));
    }

    if give_money.purpose.len() == 0 {
        return Err(ApiError::InputFailedValidation(String::from(
            "Must provide a purpose",
        )));
    }

    return Ok(());
}

async fn record_transaction(
    app_state: Arc<AppState>,
    child_name: String,
    request_trace_data: RequestTraceData,
    give_money: GiveMoney,
    transaction_type: TransactionType,
) -> Result<Json<Transaction>, ApiError> {
    let request_id = request_trace_data.get_id();
    info!(
        "[{}] {:?} called with {:?}",
        request_id, transaction_type, give_money
    );

    validate_request_body(&give_money)?;

    let mut amount = give_money.amount;
    if transaction_type == TransactionType::Spend {
        amount = amount.negate();
    }

    let persisted_transaction =
        app_state
            .get_db()
            .record_transaction_for_child(Transaction::new(
                0,
                Utc::now(),
                child_name.clone(),
                amount,
                give_money.purpose.clone(),
            ))?;

    app_state
        .queue_messages_to_active_websockets_for_child(
            child_name.clone(),
            crate::model::websocket_msg::WebSocketMsg::Transaction(persisted_transaction.clone()),
        )
        .await;

    Ok(Json(persisted_transaction))
}

pub async fn give(
    State(app_state): State<Arc<AppState>>,
    Path(child_name): Path<String>,
    Extension(request_trace_data): Extension<RequestTraceData>,
    Json(give_money): Json<GiveMoney>,
) -> Result<Json<Transaction>, ApiError> {
    record_transaction(
        app_state,
        child_name,
        request_trace_data,
        give_money,
        TransactionType::Give,
    )
    .await
}

pub async fn spend(
    State(app_state): State<Arc<AppState>>,
    Path(child_name): Path<String>,
    Extension(request_trace_data): Extension<RequestTraceData>,
    Json(give_money): Json<GiveMoney>,
) -> Result<Json<Transaction>, ApiError> {
    record_transaction(
        app_state,
        child_name,
        request_trace_data,
        give_money,
        TransactionType::Spend,
    )
    .await
}
