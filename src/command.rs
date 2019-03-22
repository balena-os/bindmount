use std::str::FromStr;

use crate::error::{Error, Result};

const MOUNT: &str = "mount";
const UNMOUNT: &str = "unmount";

pub enum Command {
    Mount,
    Unmount,
}

impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        match self {
            Command::Mount => MOUNT,
            Command::Unmount => UNMOUNT,
        }
    }
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            MOUNT => Ok(Command::Mount),
            UNMOUNT => Ok(Command::Unmount),
            _ => Err(format!("Invalid command: {}", s))?,
        }
    }
}
