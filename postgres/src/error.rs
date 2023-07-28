use std::{error, fmt};

use http_error::{HttpError, StatusCode};

#[derive(Debug)]
pub enum Error {
    MissingDatabaseUrlEnv,
    Postgres(tokio_postgres::Error),
    Build(deadpool_postgres::BuildError),
    Pool(deadpool_postgres::PoolError),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::MissingDatabaseUrlEnv => None,
            Error::Postgres(err) => Some(err),
            Error::Build(err) => Some(err),
            Error::Pool(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingDatabaseUrlEnv => f.write_str("env DATABASE_URL not set"),
            Error::Postgres(err) => err.fmt(f),
            Error::Build(_) => write!(f, "failed to build postgres connection pool"),
            Error::Pool(_) => write!(f, "failed to acquire postgress connection from pool"),
        }
    }
}

impl HttpError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Self::Postgres(err)
    }
}

impl From<deadpool_postgres::BuildError> for Error {
    fn from(err: deadpool_postgres::BuildError) -> Self {
        Self::Build(err)
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Self::Pool(err)
    }
}
