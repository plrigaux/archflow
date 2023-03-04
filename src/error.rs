use std::fmt::{self, Debug, Display};

pub enum ArchiveError {
    IoError(std::io::Error),
}

impl Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArchiveError::IoError(e) => {
                write!(f, "Archive error {:}", e)
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
        }
    }
}

impl From<std::io::Error> for ArchiveError {
    fn from(value: std::io::Error) -> Self {
        ArchiveError::IoError(value)
    }
}
