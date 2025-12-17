use std::io;

#[derive(Debug)]
pub enum Error {
    InvalidConfig,
    Os,
    PlatformNotSupported,
    Io(io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidConfig => write!(f, "invalid proxy configuration"),
            Error::Os => write!(f, "error getting proxy configuration from the Operating System"),
            Error::PlatformNotSupported => {
                write!(f, "can not read proxy configuration on this platform")
            }
            Error::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<::url::ParseError> for Error {
    fn from(_error: ::url::ParseError) -> Self {
        Error::InvalidConfig
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}
