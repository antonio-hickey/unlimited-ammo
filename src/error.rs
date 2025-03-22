#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    WatchIntervalNotSet,
    DisplayNotSet,
    BuildFailed(std::io::Error),
    FailedParsingOsString(std::ffi::OsString),
}
impl Error {
    /// Get the error message
    pub fn message(&self) -> String {
        match self {
            Self::StdIo(e) => format!("Error: Standard Input/Output\n{}", e),
            Self::WatchIntervalNotSet => {
                String::from("Error: Can't build `Watcher` without setting watch interval.")
            }
            Self::BuildFailed(e) => format!("Error: Failed to build\n{}", e),
            Self::FailedParsingOsString(e) => {
                format!("Error: Failed parsing OS Native string: {:?}", e)
            }
            Self::DisplayNotSet => String::from("Error: Log display not configured correctly."),
        }
    }
}
/// Implement the display trait for `Error`
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}
/// Implement error conversion (`std::io::Error` -> `Error`)
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::StdIo(err)
    }
}
/// Implement error conversion (`std::io::Error` -> `Error`)
impl From<std::ffi::OsString> for Error {
    fn from(err: std::ffi::OsString) -> Error {
        Error::FailedParsingOsString(err)
    }
}
