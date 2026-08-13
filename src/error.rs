use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Missing Authorization Headers")]
    MissingAuthorization,

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Invalid Credentials")]
    InvalidCredentials,

    #[error("Asset does not exist")]
    AssetDoesNotExist,

    #[error("User does not exist")]
    UserDoesNotExist,

    #[error("This username is already registered")]
    UsernameTaken,

    #[error("Quantity must be greater than zero")]
    InvalidQuantity,

    #[error("Unit price must be greater than zero")]
    InvalidUnitPrice,

    #[error("Insufficient asset quantity for this operation")]
    InsufficientQuantity,

    #[error("Internal server error")]
    InternalServerError,

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Template(#[from] askama::Error),

    #[error(transparent)]
    Jwt(#[from] jwt_simple::Error),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let error_response = ErrorResponse {
            error: self.to_string(),
        };

        let status = match self {
            Self::UsernameTaken
            | Self::MissingAuthorization
            | Self::InvalidQuantity
            | Self::InvalidUnitPrice
            | Self::InsufficientQuantity => StatusCode::BAD_REQUEST,

            Self::InvalidCredentials | Self::Unauthorized => StatusCode::UNAUTHORIZED,

            Self::AssetDoesNotExist | Self::UserDoesNotExist => StatusCode::NOT_FOUND,

            Self::InternalServerError | Self::Database(_) | Self::Template(_) | Self::Jwt(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        (status, Json(error_response)).into_response()
    }
}