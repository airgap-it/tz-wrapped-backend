use std::num::ParseIntError;

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

    #[display(fmt = "database error: {}", description)]
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

    #[display(fmt = "unauthorized")]
    Unauthorized,

    #[display(fmt = "forbidden")]
    Forbidden,

    #[display(fmt = "authentication challenge expired")]
    AuthenticationChallengeExpired,

    #[display(fmt = "unknown error")]
    Unknown,
}

impl APIError {
    pub fn name(&self) -> String {
        match self {
            APIError::NotFound => "NotFound".into(),
            APIError::InvalidSignature => "InvalidSignature".into(),
            APIError::DBError { description: _ } => "DBError".into(),
            APIError::InvalidPublicKey => "InvalidPublicKey".into(),
            APIError::Internal { description: _ } => "Internal".into(),
            APIError::InvalidOperationRequest { description: _ } => {
                "InvalidOperationRequest".into()
            }
            APIError::InvalidOperationState { description: _ } => "InvalidOperationState".into(),
            APIError::InvalidValue { description: _ } => "InvalidValue".into(),
            APIError::Unauthorized => "Unauthorized".into(),
            APIError::Forbidden => "Forbidden".into(),
            APIError::AuthenticationChallengeExpired => "AuthenticationChallengeExpired".into(),
            APIError::Unknown => "Unknown".into(),
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
            APIError::Unauthorized => StatusCode::FORBIDDEN,
            APIError::Forbidden => StatusCode::FORBIDDEN,
            APIError::AuthenticationChallengeExpired => StatusCode::BAD_REQUEST,
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

impl From<tezos::TzError> for APIError {
    fn from(value: tezos::TzError) -> Self {
        match value {
            tezos::TzError::InvalidPublicKey => APIError::InvalidPublicKey,
            tezos::TzError::InvalidSignature => APIError::InvalidSignature,
            tezos::TzError::InvalidValue { description } => APIError::InvalidValue { description },
            tezos::TzError::APIError { error } => error,
            _ => APIError::Internal {
                description: value.to_string(),
            },
        }
    }
}

impl From<bigdecimal::ParseBigDecimalError> for APIError {
    fn from(_: bigdecimal::ParseBigDecimalError) -> Self {
        APIError::InvalidValue {
            description: "cannot properly parse number into BigDecimal".into(),
        }
    }
}

impl From<num_bigint::ParseBigIntError> for APIError {
    fn from(_: num_bigint::ParseBigIntError) -> Self {
        APIError::InvalidValue {
            description: "cannot properly parse number into BigInt".into(),
        }
    }
}

impl From<lettre::smtp::error::Error> for APIError {
    fn from(_: lettre::smtp::error::Error) -> Self {
        APIError::Unknown
    }
}

impl From<lettre::error::Error> for APIError {
    fn from(_: lettre::error::Error) -> Self {
        APIError::Unknown
    }
}

impl From<lettre_email::error::Error> for APIError {
    fn from(_: lettre_email::error::Error) -> Self {
        APIError::Unknown
    }
}

impl From<native_tls::Error> for APIError {
    fn from(_: native_tls::Error) -> Self {
        APIError::Unknown
    }
}

impl From<ParseIntError> for APIError {
    fn from(_: ParseIntError) -> Self {
        APIError::Unknown
    }
}
