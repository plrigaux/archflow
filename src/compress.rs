//! <!--
//! This table shows the interpretation of the archive structure.
//!
//! <table>
//! <tr><th>Archive structure</th>
//!
//! <td>Local file header</td>
//! <td>Central directory file header</td>
//! <td>End of central directory record</td>
//! </tr>
//! <tr><th>Stream</th>
//! <td>
//! <p>Uncompress size set to 0xFFFFFFFF if >= u32::MAX</p>
//! <p>Compress size set to 0xFFFFFFFF if >= u32::MAX</p>
//! <p>ZIP64 Extra Field: No </p>
//! <p>Data Descriptor : ZIP64 format if Uncompress or Compress size >= u32::MAX</p>
//! </td>
//! <td rowspan=2>
//! <p>Uncompress size set to 0xFFFFFFFF if >= u32::MAX</p>
//! <p>Compress size set to 0xFFFFFFFF if >= u32::MAX</p>
//! <p>ZIP64 Extra Field: Yes (if Uncompress or Compress size >= u32::MAX)</p>
//! </td>
//! <td rowspan=2>
//! <p>Zip64 format if
//! <ul>
//! <li>Number of entry >= u16::MAX OR</li>
//! <li>Archive size >= u32::MAX OR</li>
//! <li>A file size >= u32::MAX OR</li>
//! </ul>
//! </p>
//! </td>
//! </tr>
//! <tr><th>Normal</th>
//! <td>
//! <p>uncompress size set to 0xFFFFFFFF if size > u32::MAX</p>
//! <p>compress size set to 0xFFFFFFFF if size > u32::MAX</p>
//! <p>ZIP64 Extra Field: Yes (if Uncompress or Compress size >= u32::MAX)</p>
//! <p>Data Descriptor : N/A </p>
//! </td>
//! </tr>
//! </table>
//! -->
//!
//!

#[cfg(feature = "std")]
pub mod std;
#[cfg(feature = "tokio")]
pub mod tokio;

mod common;

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

    /// The file creation time.
    pub last_creation_time: Option<i32>,

    /// The file modified time.
    pub last_access_time: Option<i32>,

    /// Unix permissions.
    pub unix_permissions: Option<u32>,

    /// The system of origin.
    pub system: FileCompatibilitySystem,

    /// File comment.
    pub comment: Option<&'a str>,

    /// Indicator of fize size > (u32::MAX)
    pub large_file: bool,

    /// Is the compressor will check the apparent file type
    pub detect_file_type: bool,
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
        self.unix_permissions = Some(mode & 0o777);
        self
    }

    /// Set the file comment.
    pub fn set_file_comment(mut self, comment: &'a str) -> FileOptions<'a> {
        self.comment = Some(comment);
        self
    }

    /// Set whether the new file's compressed and uncompressed size is more than 4 GiB (0xFFFFFFFF bytes).
    ///
    /// If set to `false` and the file exceeds the limit, the exra field will be replace by a data descriptor. If set to `true`,
    /// readers will require ZIP64 support and if the file does not exceed the limit, 20 B are
    /// wasted. The default is `false`.
    pub fn large_file(mut self, large: bool) -> FileOptions<'a> {
        self.large_file = large;
        self
    }

    /// Set an indicator to the archiver to detect the entry file type.
    ///
    /// The archiver will read first bytes of the entry to detect if it is a plain text or
    ///  a binary file.
    ///
    /// More information detailed there: [txtvsbin.txt](https://github.com/LuaDist/zip/blob/master/proginfo/txtvsbin.txt)
    ///
    /// Default value: true
    pub fn detect_file_type(mut self, detect_file_type: bool) -> FileOptions<'a> {
        self.detect_file_type = detect_file_type;
        self
    }

    /// Set the entry unix timestamp.
    ///
    /// The time values are in standard Unix signed-long format, indicating
    /// the number of seconds since 1 January 1970 00:00:00.
    ///
    /// all argumets are __optional__
    pub fn time_stamp(
        mut self,
        last_modification_time: Option<i32>,
        last_access_time: Option<i32>,
        last_creation_time: Option<i32>,
    ) -> FileOptions<'a> {
        self.last_modified_time = if let Some(last_modification_time) = last_modification_time {
            FileDateTime::UnixCustom(last_modification_time)
        } else {
            FileDateTime::None
        };
        self.last_access_time = last_access_time;
        self.last_creation_time = last_creation_time;
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
            unix_permissions: None,
            system: FileCompatibilitySystem::Unix,
            comment: None,
            large_file: false,
            detect_file_type: true,
            last_creation_time: None,
            last_access_time: None,
        }
    }
}
