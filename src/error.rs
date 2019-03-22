use std::{fmt, io, result};

#[derive(Debug)]
pub enum Error {
    Custom(String),
    IO(Option<String>, io::Error),
}

impl Error {
    pub fn io<S>(s: S, e: io::Error) -> Error
    where
        S: Into<String>,
    {
        Error::IO(Some(s.into()), e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IO(None, e)
    }
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::Custom(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Error {
        Error::Custom(s.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Custom(msg) => write!(f, "{}", msg),
            Error::IO(Some(msg), e) => write!(f, "{}: {}", msg, e),
            Error::IO(None, e) => write!(f, "{}", e),
        }
    }
}

pub type Result<T> = result::Result<T, Error>;
