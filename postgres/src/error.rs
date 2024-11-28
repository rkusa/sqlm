use std::{error, fmt};

use http_error::{HttpError, StatusCode};
use tracing::Span;

/// An error communicating with the Postgres server.
#[derive(Debug)]
pub struct Error {
    pub(crate) kind: ErrorKind,
    span: Span,
}

#[derive(Debug)]
pub enum ErrorKind {
    MissingDatabaseUrlEnv,
    RowNotFound,
    Postgres(tokio_postgres::Error),
    Build(deadpool_postgres::BuildError),
    Pool(deadpool_postgres::PoolError),
}

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            span: Span::current(),
        }
    }

    /// Whether this is a row not found error.
    pub fn is_row_not_found(&self) -> bool {
        matches!(self.kind, ErrorKind::RowNotFound)
    }

    /// Whether this is a duplicate key error (unique constraint violation).
    pub fn is_duplicate_key(&self) -> bool {
        if let ErrorKind::Postgres(err) = &self.kind {
            err.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION)
        } else {
            false
        }
    }

    /// Whether this is a foreign key error (foreign key constraint violation).
    pub fn is_foreign_key(&self) -> bool {
        if let ErrorKind::Postgres(err) = &self.kind {
            err.code() == Some(&tokio_postgres::error::SqlState::FOREIGN_KEY_VIOLATION)
        } else {
            false
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::MissingDatabaseUrlEnv => None,
            ErrorKind::RowNotFound => None,
            ErrorKind::Postgres(err) => Some(err),
            ErrorKind::Build(err) => Some(err),
            ErrorKind::Pool(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::MissingDatabaseUrlEnv => f.write_str("env DATABASE_URL not set"),
            ErrorKind::RowNotFound => f.write_str("No rows returned, but at least one expected"),
            ErrorKind::Postgres(err) => err.fmt(f),
            ErrorKind::Build(_) => write!(f, "failed to build postgres connection pool"),
            ErrorKind::Pool(_) => write!(f, "failed to acquire postgres connection from pool"),
        }
    }
}

impl HttpError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn span(&self) -> Option<&tracing::Span> {
        Some(&self.span)
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Self::new(ErrorKind::Postgres(err))
    }
}

impl From<deadpool_postgres::BuildError> for Error {
    fn from(err: deadpool_postgres::BuildError) -> Self {
        Self::new(ErrorKind::Build(err))
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Self::new(ErrorKind::Pool(err))
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self::new(kind)
    }
}
