use std::{error, fmt};

use http_error::{HttpError, StatusCode};
use tracing::Span;

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
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

    pub fn is_row_not_found(&self) -> bool {
        matches!(self.kind, ErrorKind::RowNotFound)
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
            ErrorKind::Pool(_) => write!(f, "failed to acquire postgress connection from pool"),
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
