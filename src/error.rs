use sdl3::Error as SdlError;

use crate::net::error::NetError;

/// Result type for application actions.
pub(crate) type Result<T> = std::result::Result<T, AppError>;

/// Error codes for various connection actions.
#[derive(Debug, PartialEq)]
pub enum AppError {
    Net(NetError),  // Network error occurred.
    Sdl(SdlError),  // SDL error occurred.
    Window(String), // Window error occurred.
}

impl std::error::Error for AppError {}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Net(why) => write!(f, "Network Error: {why}"),
            AppError::Sdl(why) => write!(f, "SDL Error: {why}"),
            AppError::Window(why) => write!(f, "Window Error: {why}"),
        }
    }
}
