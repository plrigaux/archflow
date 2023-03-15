use crate::{
    compression::{CompressionMethod, Level},
    types::{FileCompatibilitySystem, FileDateTime},
};

/// Metadata for a file to be written
#[derive(Clone)]
pub struct FileOptions {
    pub compressor: CompressionMethod,
    pub compression_level: Level,
    pub last_modified_time: FileDateTime,
    pub permissions: Option<u32>,
    pub system: FileCompatibilitySystem,
}

impl FileOptions {
    /// Set the compression method for the new file
    ///
    /// The default is `CompressionMethod::Deflated`. If the deflate compression feature is
    /// disabled, `CompressionMethod::Stored` becomes the default.
    pub fn compression_method(mut self, method: CompressionMethod) -> FileOptions {
        self.compressor = method;
        self
    }

    /// Set the compression level for the new file
    ///
    /// `None` value specifies default compression level.
    ///
    /// Range of values depends on compression method:
    /// * `Deflated`: 0 - 9. Default is 6
    /// * `Bzip2`: 0 - 9. Default is 6
    /// * `Zstd`: -7 - 22, with zero being mapped to default level. Default is 3
    /// * others: only `None` is allowed
    pub fn compression_level(mut self, level: Level) -> FileOptions {
        self.compression_level = level;
        self
    }

    /// Set the last modified time
    ///
    /// The default is the current timestamp if the 'time' feature is enabled, and 1980-01-01
    /// otherwise
    pub fn last_modified_time(mut self, mod_time: FileDateTime) -> FileOptions {
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
    pub fn unix_permissions(mut self, mode: u32) -> FileOptions {
        self.permissions = Some(mode & 0o777);
        self
    }
}

impl Default for FileOptions {
    /// Construct a new FileOptions object
    fn default() -> Self {
        Self {
            compressor: CompressionMethod::Deflate(),
            compression_level: Level::Default,
            last_modified_time: FileDateTime::Now,
            permissions: None,
            system: FileCompatibilitySystem::Unix,
        }
    }
}
