use serde::Serialize;
use std::fmt::Display;

#[derive(Debug, Serialize)]
pub enum Error {
    NotFound(ErrorDetails),
    BadRequest(ErrorDetails),
    Unauthorized(ErrorDetails),
    InternalError(ErrorDetails),
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    code: String,
    message: String,
}

impl Error {
    pub fn not_found(message: &str) -> Self {
        Self::NotFound(ErrorDetails {
            code: "not_found".into(),
            message: message.into(),
        })
    }

    pub fn bad_request(message: &str) -> Self {
        Self::BadRequest(ErrorDetails {
            code: "bad_request".into(),
            message: message.into(),
        })
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::Unauthorized(ErrorDetails {
            code: "unauthorized".into(),
            message: message.into(),
        })
    }

    pub fn internal(message: &str) -> Self {
        Self::InternalError(ErrorDetails {
            code: "internal".into(),
            message: message.into(),
        })
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::BadRequest(d)
            | Self::InternalError(d)
            | Self::NotFound(d)
            | Self::Unauthorized(d) => d.message.as_str(),
        };

        f.write_str(&format!("{}", msg))
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        use sqlx::Error::*;
        match &value {
            Tls(_) | Protocol(_) | Io(_) | PoolClosed | WorkerCrashed | PoolTimedOut => {
                Error::internal(&value.to_string())
            }
            RowNotFound => Error::not_found(&value.to_string()),
            _ => Error::bad_request(&value.to_string()),
        }
    }
}
