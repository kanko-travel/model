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
        f.write_str(&format!("{:?}", self))
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                Error::not_found("no rows returned from query that expected at least one row")
            }
            _ => Error::internal(&err.to_string()),
        }
    }
}
