use axum::{body::Body, http::Request};

use crate::model::error::ApiError;

pub async fn handler_404(request: Request<Body>) -> ApiError {
    ApiError::PathNotFound(request.uri().to_string())
}
