use core::error::Error;
use core::fmt;

use wdk_sys::{KIRQL, NTSTATUS};

#[derive(Debug)]
pub enum RuntimeError {
    Failure(NTSTATUS),
    InvalidIRQL(KIRQL),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failure(status) => write!(f, "Operation failed with status {status}"),
            Self::InvalidIRQL(irql) => write!(f, "Invalid IRQL {irql}"),
        }
    }
}

impl Error for RuntimeError {}
