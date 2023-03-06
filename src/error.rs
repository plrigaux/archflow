use std::fmt::{self, Debug, Display};

use crate::compression::CompressionMethod;

pub enum ArchiveError {
    IoError(std::io::Error),
    UnsuportedCompressionLevel(CompressionMethod),
    UnsuportedCompressionMethod(u32),
}

impl Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArchiveError::IoError(e) => {
                write!(f, "Archive error {:}", e)
            }
            ArchiveError::UnsuportedCompressionLevel(method) => {
                write!(f, "Archive level error for method {:}", method)
            }
            ArchiveError::UnsuportedCompressionMethod(val) => {
                write!(f, "The compression method code '{:}' is not supported", val)
            }
        }
    }
}

impl Debug for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArchiveError::IoError(e) => {
                write!(f, "Archive error {:?}", e)
            }
            ArchiveError::UnsuportedCompressionLevel(method) => {
                write!(f, "Archive level error for method {:?}", method)
            }
            ArchiveError::UnsuportedCompressionMethod(val) => write!(
                f,
                "The compression method code '{:?}' is not supported",
                val
            ),
        }
    }
}

impl From<std::io::Error> for ArchiveError {
    fn from(value: std::io::Error) -> Self {
        ArchiveError::IoError(value)
    }
}
