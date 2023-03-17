#[cfg(feature = "std")]
pub mod std;
#[cfg(feature = "tokio")]
pub mod tokio;

use crate::{
    compression::{CompressionMethod, Level},
    types::{FileCompatibilitySystem, FileDateTime},
};

/// Metadata for a file to be archived
#[derive(Clone)]
pub struct FileOptions<'a> {
    /// The file's selected compression method.
    pub compression_method: CompressionMethod,

    /// The compression method's level.
    pub compression_level: Level,

    /// The file modified time.
    pub last_modified_time: FileDateTime,

    /// Unix permissions.
    pub permissions: Option<u32>,

    /// The system of origin.
    pub system: FileCompatibilitySystem,

    /// File comment.
    pub comment: Option<&'a str>,
}

impl<'a> FileOptions<'a> {
    /// Set the compression method for the new file
    ///
    /// The default is `CompressionMethod::Deflated`.
    ///
    pub fn compression_method(mut self, method: CompressionMethod) -> FileOptions<'a> {
        self.compression_method = method;
        self
    }

    /// Set the compression level for the new file
    pub fn compression_level(mut self, level: Level) -> FileOptions<'a> {
        self.compression_level = level;
        self
    }

    /// Set the last modified time
    ///
    /// The default is the current timestamp
    pub fn last_modified_time(mut self, mod_time: FileDateTime) -> FileOptions<'a> {
        self.last_modified_time = mod_time;
        self
    }

    /// Set the permissions for the new file.
    ///
    /// The format is represented with unix-style permissions.
    /// The default is `0o644`, which represents `rw-r--r--` for files,
    /// and `0o755`, which represents `rwxr-xr-x` for directories.
    ///
    /// This method only preserves the file permissions bits (via a `& 0o777`) and discards
    /// higher file mode bits. So it cannot be used to denote an entry as a directory,
    /// symlink, or other special file type.
    pub fn unix_permissions(mut self, mode: u32) -> FileOptions<'a> {
        self.permissions = Some(mode & 0o777);
        self
    }

    /// Set the file comment.
    pub fn set_file_comment(mut self, comment: &'a str) -> FileOptions<'a> {
        self.comment = Some(comment);
        self
    }
}

impl<'a> Default for FileOptions<'a> {
    /// Construct a new FileOptions object
    fn default() -> Self {
        Self {
            compression_method: CompressionMethod::Deflate(),
            compression_level: Level::Default,
            last_modified_time: FileDateTime::Now,
            permissions: None,
            system: FileCompatibilitySystem::Unix,
            comment: None,
        }
    }
}
