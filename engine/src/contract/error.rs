use super::context::ContextError;
use thiserror::Error;

/// Typed internal error vocabulary shared by source, engine, storage and RPC
/// contract code. Callers should map `code()` to external protocols instead of
/// exposing implementation details from `Display` to untrusted clients.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContractError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid repository context: {0}")]
    InvalidContext(String),
    #[error("unsupported input: {0}")]
    Unsupported(String),
    #[error("unresolved reference: {0}")]
    Unresolved(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("resource unavailable: {0}")]
    Unavailable(String),
    #[error("operation limit exceeded: {0}")]
    LimitExceeded(String),
    #[error("operation timed out: {0}")]
    Timeout(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("internal failure: {0}")]
    Internal(String),
}

impl ContractError {
    /// Stable, non-sensitive machine-readable category for logs and RPC mapping.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "INVALID_ARGUMENT",
            Self::InvalidContext(_) => "INVALID_CONTEXT",
            Self::Unsupported(_) => "UNSUPPORTED",
            Self::Unresolved(_) => "UNRESOLVED_REFERENCE",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Unavailable(_) => "UNAVAILABLE",
            Self::LimitExceeded(_) => "LIMIT_EXCEEDED",
            Self::Timeout(_) => "TIMEOUT",
            Self::Conflict(_) => "CONFLICT",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::Internal(_) => "INTERNAL",
        }
    }

    /// Retry only failures whose category is plausibly transient. The caller
    /// must still apply its centralized retry budget/deadline.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Unavailable(_) | Self::Timeout(_))
    }
}

impl From<ContextError> for ContractError {
    fn from(error: ContextError) -> Self {
        Self::InvalidContext(error.to_string())
    }
}

impl From<serde_json::Error> for ContractError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ContractError;

    #[test]
    fn error_codes_are_stable_and_do_not_require_display_parsing() {
        assert_eq!(ContractError::NotFound("symbol".into()).code(), "NOT_FOUND");
        assert_eq!(ContractError::Timeout("query".into()).code(), "TIMEOUT");
    }

    #[test]
    fn retryability_is_limited_to_transient_categories() {
        assert!(ContractError::Unavailable("database".into()).is_retryable());
        assert!(ContractError::Timeout("query".into()).is_retryable());
        assert!(!ContractError::Conflict("state".into()).is_retryable());
        assert!(!ContractError::LimitExceeded("depth".into()).is_retryable());
    }
}
