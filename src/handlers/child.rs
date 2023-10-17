use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    appstate::AppState,
    middleware::request_tracing::RequestTraceData,
    model::{amount::Amount, error::ApiError, transaction::Transaction},
};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ChildAccountResponse {
    pub child_name: String,
    pub balance: Amount,
    pub transactions: Vec<Transaction>,
}

pub async fn get_child(
    State(app_state): State<Arc<AppState>>,
    Path(child_name): Path<String>,
    Extension(request_trace_data): Extension<RequestTraceData>,
) -> Result<Json<ChildAccountResponse>, ApiError> {
    info!("[{}] get_child {}", request_trace_data.get_id(), child_name);

    let transactions = app_state
        .get_db()
        .get_transactions_for_child(child_name.clone())?;

    let balance = if transactions.len() == 0 {
        Amount::from_pence(0)
    } else {
        app_state
            .get_db()
            .get_account_balance_for_child(child_name.clone())?
    };

    let response = ChildAccountResponse {
        child_name,
        balance,
        transactions: transactions,
    };

    Ok(Json(response))
}
