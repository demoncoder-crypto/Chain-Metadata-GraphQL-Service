use thiserror::Error;
use async_graphql::{Error as GraphQLError, ErrorExtensions};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("GraphQL execution error: {0}")]
    GraphQLExecution(String),

    #[error("Service error: {0}")]
    ServiceError(String),
}

impl ErrorExtensions for AppError {
    fn extend(&self) -> GraphQLError {
        GraphQLError::new(format!("{}", self)).extend_with(|_err, e| match self {
            AppError::NotFound(reason) => e.set("code", "NOT_FOUND").set("reason", reason.clone()),
            AppError::Config(s) => e.set("code", "CONFIG_ERROR").set("details", s.to_string()),
            AppError::Internal(s) => e.set("code", "INTERNAL_SERVER_ERROR").set("details", s.clone()),
            AppError::GraphQLExecution(s) => e.set("code", "GRAPHQL_EXECUTION_ERROR").set("details", s.clone()),
            AppError::ServiceError(s) => e.set("code", "SERVICE_ERROR").set("details", s.clone()),
            AppError::Io(s) => e.set("code", "IO_ERROR").set("details", s.to_string()),
        })
    }
}

// Allow converting AppError to FieldResult (which is Result<T, GraphQLError>)
pub type Result<T, E = AppError> = std::result::Result<T, E>;

impl From<AppError> for GraphQLError {
    fn from(err: AppError) -> Self {
        err.extend()
    }
} 