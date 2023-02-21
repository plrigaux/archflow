use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use crc32fast::Hasher;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::async_write_wrapper::AsyncWriteWrapper;
use crate::compression::Compressor;
use crate::constants::{
    CENTRAL_DIRECTORY_END_SIGNATURE, CENTRAL_DIRECTORY_ENTRY_BASE_SIZE,
    CENTRAL_DIRECTORY_ENTRY_SIGNATURE, DESCRIPTOR_SIZE, END_OF_CENTRAL_DIRECTORY_SIZE,
    FILE_HEADER_BASE_SIZE,
};
use std::io::Error as IoError;

#[derive(Debug)]
struct ArchiveFileEntry {
    name: String,
    compressed_size: u32, // Compressed size.
    original_size: u32,
    crc: u32,
    offset: u32,
    datetime: (u16, u16),
    compressor: Compressor,
}

pub const DEFAULT_VERSION: u8 = 46;
pub const UNIX: u8 = 3;
pub const VERSION_MADE_BY: u16 = (UNIX as u16) << 8 | DEFAULT_VERSION as u16;

impl ArchiveFileEntry {
    pub fn version_needed(&self) -> u16 {
        // higher versions matched first
        match self.compressor {
            Compressor::BZip2() => 46,
            _ => 20,
        }
    }

    pub fn version_made_by(&self) -> u16 {
        VERSION_MADE_BY
    }
}

/// The (timezone-less) date and time that will be written in the archive alongside the file.
///
/// Use `FileDateTime::Zero` if the date and time are insignificant. This will set the value to 0 which is 1980, January 1th, 12AM.  
/// Use `FileDateTime::Custom` if you need to set a custom date and time.  
/// Use `FileDateTime::now()` if you want to use the current date and time (`chrono-datetime` feature required).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FileDateTime {
    /// 1980, January 1th, 12AM.
    Zero,
    /// (year, month, day, hour, minute, second)
    Custom {
        year: u16,
        month: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
    },
}

impl FileDateTime {
    fn tuple(&self) -> (u16, u16, u16, u16, u16, u16) {
        match self {
            FileDateTime::Zero => Default::default(),
            &FileDateTime::Custom {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => (year, month, day, hour, minute, second),
        }
    }

    fn ms_dos(&self) -> (u16, u16) {
        let (year, month, day, hour, min, sec) = self.tuple();
        (
            day | month << 5 | year.saturating_sub(1980) << 9,
            (sec / 2) | min << 5 | hour << 11,
        )
    }

    /// Use the local date and time of the system.
    pub fn now() -> Self {
        Self::from_chrono_datetime(Local::now())
    }

    /// Use a custom date and time.
    pub fn from_chrono_datetime<Tz: TimeZone>(datetime: DateTime<Tz>) -> Self {
        Self::Custom {
            year: datetime.year() as u16,
            month: datetime.month() as u16,
            day: datetime.day() as u16,
            hour: datetime.hour() as u16,
            minute: datetime.minute() as u16,
            second: datetime.second() as u16,
        }
    }
}

macro_rules! header {
    [$capacity:expr; $($elem:expr),*$(,)?] => {
        {
            let mut header = Vec::with_capacity($capacity);
            $(
                header.extend_from_slice(&$elem.to_le_bytes());
            )*
            header
        }
    };
}

#[derive(Debug)]
pub struct Archive<W: tokio::io::AsyncWrite + Unpin> {
    sink: AsyncWriteWrapper<W>,
    files_info: Vec<ArchiveFileEntry>,
    written: u32,
}

impl<W: tokio::io::AsyncWrite + Unpin> Archive<W> {
    /// Create a new zip archive, using the underlying `AsyncWrite` to write files' header and payload.
    pub fn new(sink_: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink: AsyncWriteWrapper::new(sink_),
            files_info: Vec::new(),
            written: 0,
        }
    }

    pub fn retrieve_writer(self) -> W {
        self.sink.retrieve_writer()
    }

    pub fn get_archive_size(&self) -> usize {
        self.sink.get_compress_length()
    }

    pub fn update_written(&mut self, nb_bytes: u32) {
        //println!("written bytes: {}", nb_bytes);
        self.written += nb_bytes;
    }
    /// Append a new file to the archive using the provided name, date/time and `AsyncRead` object.  
    /// Filename must be valid UTF-8. Some (very) old zip utilities might mess up filenames during extraction if they contain non-ascii characters.  
    /// File's payload is not compressed and is given `rw-r--r--` permissions.
    ///
    /// # Error
    ///
    /// This function will forward any error found while trying to read from the file stream or while writing to the underlying sink.
    ///
    /// # Features
    ///
    /// Requires `tokio-async-io` feature. `futures-async-io` is also available.
    pub async fn append<R>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        self.append_base(name, datetime, reader, Compressor::Storer())
            .await?;

        Ok(())
    }

    pub async fn append_file<R>(
        &mut self,
        file_name: &str,
        datetime: FileDateTime,
        compressor: Compressor,
        reader: &mut R,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        self.append_base(file_name.to_owned(), datetime, reader, compressor)
            .await?;

        Ok(())
    }

    pub async fn appendzip<R>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        self.append_base(name, datetime, reader, Compressor::Deflater())
            .await?;
        Ok(())
    }

    pub async fn append_bzip<R>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        self.append_base(name, datetime, reader, Compressor::BZip2())
            .await?;
        Ok(())
    }

    async fn append_base<R>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
        compressor: Compressor,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        let (date, time) = datetime.ms_dos();
        let offset = self.written;

        let compression_method = compressor.compression_method();

        let mut header = header![
            FILE_HEADER_BASE_SIZE + name.len();
            0x04034b50u32,          // Local file header signature.
            compressor.version_needed(),                  // Version needed to extract.
            1u16 << 3 | 1 << 11,    // General purpose flag (temporary crc and sizes + UTF-8 filename).
            compression_method,     // Compression method .
            time,                   // Modification time.
            date,                   // Modification date.
            0u32,                   // Temporary CRC32.
            0u32,                   // Temporary compressed size.
            0u32,                   // Temporary uncompressed size.
            name.len() as u16,      // Filename length.
            0u16,                   // Extra field length.
        ];
        header.extend_from_slice(name.as_bytes()); // Filename.
        self.sink.write_all(&header).await?;
        //self.sink.flush().await?;
        self.update_written(header.len() as u32);
        let mut hasher = Hasher::new();
        let cur_size = self.sink.get_compress_length();

        let total_read = compressor
            .compress(&mut self.sink, reader, &mut hasher)
            .await?;

        //self.sink.flush().await?;
        let total_compress = self.sink.get_compress_length() - cur_size;
        //println!("total_read {:?}", total_read);
        /*         println!(
            "total_compress {:?}, written: {:?} cur_size {:?}",
            total_compress,
            self.sink.get_compress_length(),
            cur_size
        ); */

        /*         println!(
            "read - compress = save {:?} bytes",
            (total_read as i64) - (total_compress as i64)
        ); */
        //self.update_written(total_read as u32);
        self.update_written(total_compress as u32);

        let crc = hasher.finalize();

        let descriptor = header![
            DESCRIPTOR_SIZE;
            0x08074b50u32,      // Data descriptor signature.
            crc,                // CRC32.
            total_compress as u32,  // Compressed size.
            total_read as u32,  // Uncompressed size.
        ];
        self.sink.write_all(&descriptor).await?;

        self.update_written(descriptor.len() as u32);

        self.files_info.push(ArchiveFileEntry {
            name,
            original_size: total_read as u32,
            compressed_size: total_compress as u32,
            crc,
            offset,
            datetime: (date, time),
            compressor,
        });

        Ok(())
    }

    /// Finalize the archive by writing the necessary metadata to the end of the archive.
    ///
    /// # Error
    ///
    /// This function will forward any error found while trying to read from the file stream or while writing to the underlying sink.
    ///
    /// # Features
    ///
    /// Requires `tokio-async-io` feature. `futures-async-io` is also available.
    pub async fn finalize(&mut self) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
    {
        let mut central_directory_size = 0;
        for file_info in &self.files_info {
            let mut entry = header![
                CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + file_info.name.len();
                CENTRAL_DIRECTORY_ENTRY_SIGNATURE,                  // Central directory entry signature.
                file_info.version_made_by(),                      // Version made by.
                file_info.version_needed(),                          // Version needed to extract.
                1u16 << 3 | 1 << 11,            // General purpose flag (temporary crc and sizes + UTF-8 filename).
                file_info.compressor.compression_method(),       // Compression method .
                file_info.datetime.1,           // Modification time.
                file_info.datetime.0,           // Modification date.
                file_info.crc,                  // CRC32.
                file_info.compressed_size,          // Compressed size.
                file_info.original_size,          // Uncompressed size.
                file_info.name.len() as u16,    // Filename length.
                0u16,                           // Extra field length.
                0u16,                           // File comment length.
                0u16,                           // File's Disk number.
                0u16,                           // Internal file attributes.
                (0o100000u32 | 0o0000400 | 0o0000200 | 0o0000040 | 0o0000004) << 16, // External file attributes (regular file / rw-r--r--).
                file_info.offset,        // Offset from start of file to local file header.
            ];
            entry.extend_from_slice(file_info.name.as_bytes()); // Filename.
            self.sink.write_all(&entry).await?;
            central_directory_size += entry.len();
        }

        /*         let end_of_central_directory = header![
            END_OF_CENTRAL_DIRECTORY_SIZE;
            CENTRAL_DIRECTORY_END_SIGNATURE,                  // End of central directory signature.
            0u16,                           // Number of this disk.
            0u16,                           // Number of the disk where central directory starts.
            self.files_info.len() as u16,   // Number of central directory records on this disk.
            self.files_info.len() as u16,   // Total number of central directory records.
            central_directory_size as u32,  // Size of central directory.
            self.written as u32,            // Offset from start of file to central directory.
            0u16,                           // Comment length.
        ];
        self.sink.write_all(&end_of_central_directory).await?; */

        let dir_end = CentralDirectoryEnd {
            disk_number: 0,
            disk_with_central_directory: 0,
            number_of_files_on_this_disk: self.files_info.len() as u16,
            number_of_files: self.files_info.len() as u16,
            central_directory_size: central_directory_size as u32,
            central_directory_offset: self.written,
            zip_file_comment_len: 0,
        };

        dir_end.write(&mut self.sink).await?;

        //println!("CentralDirectoryEnd {:#?}", dir_end);
        Ok(())
    }
}

#[derive(Debug)]
pub struct CentralDirectoryEnd {
    pub disk_number: u16,
    pub disk_with_central_directory: u16,
    pub number_of_files_on_this_disk: u16,
    pub number_of_files: u16,
    pub central_directory_size: u32,
    pub central_directory_offset: u32,
    pub zip_file_comment_len: u16,
}

impl CentralDirectoryEnd {
    async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut AsyncWriteWrapper<W>,
    ) -> Result<(), IoError> {
        let end_of_central_directory = header![
            END_OF_CENTRAL_DIRECTORY_SIZE;
            CENTRAL_DIRECTORY_END_SIGNATURE,                  // End of central directory signature.
            self.disk_number,                           // Number of this disk.
            self.disk_with_central_directory,                           // Number of the disk where central directory starts.
            self.number_of_files_on_this_disk,   // Number of central directory records on this disk.
            self.number_of_files,                // Total number of central directory records.
            self.central_directory_size,         // Size of central directory.
            self.central_directory_offset,            // Offset from start of file to central directory.
            self.zip_file_comment_len,                           // Comment length.

        ];

        writer.write_all(&end_of_central_directory).await?;

        Ok(())
    }
}
