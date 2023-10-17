use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use db::Db;

use tower::ServiceBuilder;

use crate::appstate::AppState;

pub mod appstate;
pub mod db;
pub mod handlers;
pub mod middleware;
pub mod model;

pub fn router(db: Db) -> Router {
    let app_state = Arc::new(AppState::new(db));

    Router::new()
        .route("/child/:child_name", get(crate::handlers::child::get_child))
        .route(
            "/child/:child_name/give",
            post(crate::handlers::record_transaction::give),
        )
        .route(
            "/child/:child_name/spend",
            post(crate::handlers::record_transaction::spend),
        )
        .route(
            "/child/:child_name/notifications",
            get(crate::handlers::websocket::accept_websocket),
        )
        .fallback(crate::handlers::path_not_found::handler_404)
        .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
            crate::middleware::request_tracing::request_tracing,
        )))
        .with_state(app_state)
}