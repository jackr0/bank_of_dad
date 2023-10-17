use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use log::{error, warn};
use rusqlite::Error;
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    InternalError(String),
    InputFailedValidation(String),
    PathNotFound(String),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    reason: String,
}

impl From<rusqlite::Error> for ApiError {
    fn from(value: Error) -> ApiError {
        error!("rusqlite error: {}", value);
        ApiError::InternalError(String::from("Internal Error"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InternalError(public_reason) => {
                warn!(
                    "INTERNAL_SERVER_ERROR response with public_reason={}",
                    public_reason
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        reason: public_reason,
                    }),
                )
            }
            Self::InputFailedValidation(public_reason) => {
                warn!("BAD_REQUEST response with public_reason={}", public_reason);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        reason: public_reason,
                    }),
                )
            }
            Self::PathNotFound(path) => {
                let public_reason = format!("Requested path '{}' not found", path);
                warn!("NOT_FOUND response with public_reason={}", public_reason);
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        reason: public_reason,
                    }),
                )
            }
        }
        .into_response()
    }
}
