use crate::net::NetError;

/// Result type for application actions.
pub(crate) type Result<T> = std::result::Result<T, AppError>;

/// Error codes for various connection actions.
#[derive(Debug, PartialEq)]
pub enum AppError {
    NetError(NetError), // Network error occurred.
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NetError(why) => write!(f, "Network Error: {why}"),
        }
    }
}
