use actix_web::{error::BlockingError, http::StatusCode, HttpResponse, ResponseError};
use derive_more::{Display, Error};
use serde::Serialize;

use crate::tezos;

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

#[derive(Error, Display, Debug)]
pub enum APIError {
    #[display(fmt = "not found")]
    NotFound,

    #[display(fmt = "invalid signature")]
    InvalidSignature,

    #[display(fmt = "Database error: {}", description)]
    DBError { description: String },

    #[display(fmt = "invalid public key")]
    InvalidPublicKey,

    #[display(fmt = "internal: {}", description)]
    Internal { description: String },

    #[display(fmt = "invalid operation request: {}", description)]
    InvalidOperationRequest { description: String },

    #[display(fmt = "invalid value: {}", description)]
    InvalidValue { description: String },

    #[display(fmt = "invalid operation state: {}", description)]
    InvalidOperationState { description: String },

    #[display(fmt = "unknown error")]
    Unknown,
}

impl APIError {
    pub fn name(&self) -> String {
        match self {
            APIError::NotFound => "NotFound".to_string(),
            APIError::InvalidSignature => "InvalidSignature".to_string(),
            APIError::DBError { description: _ } => "DBError".to_string(),
            APIError::InvalidPublicKey => "InvalidPublicKey".to_string(),
            APIError::Internal { description: _ } => "Internal".to_string(),
            APIError::InvalidOperationRequest { description: _ } => {
                "InvalidOperationRequest".to_string()
            }
            APIError::InvalidOperationState { description: _ } => {
                "InvalidOperationState".to_string()
            }
            APIError::InvalidValue { description: _ } => "InvalidValue".to_string(),
            APIError::Unknown => "Unknown".to_string(),
        }
    }
}

impl ResponseError for APIError {
    fn status_code(&self) -> StatusCode {
        match self {
            APIError::NotFound => StatusCode::NOT_FOUND,
            APIError::InvalidSignature => StatusCode::BAD_REQUEST,
            APIError::DBError { description: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            APIError::InvalidPublicKey => StatusCode::BAD_REQUEST,
            APIError::Internal { description: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            APIError::InvalidOperationRequest { description: _ } => StatusCode::BAD_REQUEST,
            APIError::InvalidOperationState { description: _ } => StatusCode::BAD_REQUEST,
            APIError::InvalidValue { description: _ } => StatusCode::BAD_REQUEST,
            APIError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}

impl From<diesel::result::Error> for APIError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => APIError::NotFound,
            _ => APIError::DBError {
                description: error.to_string(),
            },
        }
    }
}

impl From<BlockingError<diesel::result::Error>> for APIError {
    fn from(error: BlockingError<diesel::result::Error>) -> Self {
        match error {
            BlockingError::Error(db_error) => APIError::from(db_error),
            BlockingError::Canceled => APIError::DBError {
                description: error.to_string(),
            },
        }
    }
}

impl From<BlockingError<APIError>> for APIError {
    fn from(error: BlockingError<APIError>) -> Self {
        match error {
            BlockingError::Error(api_error) => api_error,
            BlockingError::Canceled => APIError::DBError {
                description: format!("{}", error),
            },
        }
    }
}

impl From<r2d2::Error> for APIError {
    fn from(error: r2d2::Error) -> Self {
        APIError::DBError {
            description: error.to_string(),
        }
    }
}

impl From<tezos::micheline::TzError> for APIError {
    fn from(value: tezos::micheline::TzError) -> Self {
        APIError::Internal {
            description: value.to_string(),
        }
    }
}
