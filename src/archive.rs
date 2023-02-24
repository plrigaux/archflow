use crc32fast::Hasher;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::async_write_wrapper::AsyncWriteWrapper;
use crate::compression::Compressor;
use crate::constants::{
    CENTRAL_DIRECTORY_END_SIGNATURE, CENTRAL_DIRECTORY_ENTRY_BASE_SIZE,
    CENTRAL_DIRECTORY_ENTRY_SIGNATURE, DATA_DESCRIPTOR_SIGNATURE, DESCRIPTOR_SIZE,
    END_OF_CENTRAL_DIRECTORY_SIZE, FILE_HEADER_BASE_SIZE, LOCAL_FILE_HEADER_SIGNATURE,
};
use crate::descriptor::ArchiveDescriptor;
use crate::types::{ArchiveFileEntry, FileDateTime};
use std::io::Error as IoError;

pub const DEFAULT_VERSION: u8 = 46;
pub const UNIX: u8 = 3;
pub const VERSION_MADE_BY: u16 = (UNIX as u16) << 8 | DEFAULT_VERSION as u16;

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
    written_bytes_count: u32,
}

impl<W: tokio::io::AsyncWrite + Unpin> Archive<W> {
    /// Create a new zip archive, using the underlying `AsyncWrite` to write files' header and payload.
    pub fn new(sink_: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink: AsyncWriteWrapper::new(sink_),
            files_info: Vec::new(),
            written_bytes_count: 0,
        }
    }

    pub fn retrieve_writer(self) -> W {
        self.sink.retrieve_writer()
    }

    pub fn get_archive_size(&self) -> usize {
        self.sink.get_compress_length()
    }

    pub fn update_written_bytes_count(&mut self, nb_bytes: u32) {
        self.written_bytes_count += nb_bytes;
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
        self.append_base(file_name, datetime, reader, compressor)
            .await?;

        Ok(())
    }

    pub async fn append_file_no_extend<R>(
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
        self.append_base_local_headed(file_name, datetime, reader, compressor)
            .await?;

        Ok(())
    }

    async fn append_base<R>(
        &mut self,
        file_name: &str,
        datetime: FileDateTime,
        reader: &mut R,
        compressor: Compressor,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        let (date, time) = datetime.ms_dos();
        let offset = self.written_bytes_count;

        let compression_method = compressor.compression_method();

        let file_len: u16 = file_name.as_bytes().len() as u16;

        let extra_field_length = 0u16;
        let version_needed = compressor.version_needed();

        let file_nameas_bytes = file_name.as_bytes();
        let file_name_as_bytes_own = file_nameas_bytes.to_owned();
        let file_name_len = file_name_as_bytes_own.len() as u16;

        let mut general_purpose_flags: u16 = 1 << 3;
        if file_name_as_bytes_own.len() > file_name.len() {
            general_purpose_flags |= 1 << 11; //set utf8 flag
        }

        /*         let mut header = header![
            FILE_HEADER_BASE_SIZE + file_len as usize;
            LOCAL_FILE_HEADER_SIGNATURE,          // Local file header signature.
            version_needed,                  // Version needed to extract.
            general_purpose_flags,    // General purpose flag (temporary crc and sizes + UTF-8 filename).
            compression_method,     // Compression method .
            time,                   // Modification time.
            date,                   // Modification date.
            0u32,                   // Temporary CRC32.
            0u32,                   // Temporary compressed size.
            0u32,                   // Temporary uncompressed size.
            file_len,      // Filename length.
            extra_field_length,                   // Extra field length.
        ]; */

        let mut file_header =
            ArchiveDescriptor::new(FILE_HEADER_BASE_SIZE + file_name_len as usize);
        file_header.write_u32(LOCAL_FILE_HEADER_SIGNATURE);
        file_header.write_u16(version_needed);
        file_header.write_u16(general_purpose_flags);
        file_header.write_u16(compression_method);
        file_header.write_u16(time);
        file_header.write_u16(date);
        file_header.write_u32(0u32);
        file_header.write_u32(0u32);
        file_header.write_u32(0u32);
        file_header.write_u16(file_name_len);
        file_header.write_u16(extra_field_length);
        file_header.write_bytes(&file_name_as_bytes_own);
        let file_header_bytes = file_header.finish();

        self.sink.write_all(&file_header_bytes).await?;
        //self.sink.flush().await?;
        self.update_written_bytes_count(file_header_bytes.len() as u32);

        let mut hasher = Hasher::new();
        let cur_size = self.sink.get_compress_length();

        let uncompressed_size = compressor
            .compress(&mut self.sink, reader, &mut hasher)
            .await?;

        //self.sink.flush().await?;
        let compressed_size = self.sink.get_compress_length() - cur_size;

        self.update_written_bytes_count(compressed_size as u32);

        let crc32 = hasher.finalize();

        let descriptor = header![
            DESCRIPTOR_SIZE;
            DATA_DESCRIPTOR_SIGNATURE,      // Data descriptor signature.
            crc32,                // CRC32.
            compressed_size as u32,  // Compressed size.
            uncompressed_size as u32,  // Uncompressed size.
        ];
        self.sink.write_all(&descriptor).await?;

        self.update_written_bytes_count(descriptor.len() as u32);

        self.files_info.push(ArchiveFileEntry {
            file_name_as_bytes: file_name_as_bytes_own,
            file_name_len: file_len,
            uncompressed_size: uncompressed_size as u32,
            compressed_size: compressed_size as u32,
            crc: crc32,
            offset,
            last_mod_file_time: time,
            last_mod_file_date: date,
            compressor,
            general_purpose_flags,
            extra_field_length,
            version_needed,
            compression_method,
        });

        Ok(())
    }

    async fn append_base_local_headed<R>(
        &mut self,
        file_name: &str,
        datetime: FileDateTime,
        reader: &mut R,
        compressor: Compressor,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        let (date, time) = datetime.ms_dos();
        let offset = self.written_bytes_count;

        let compression_method = compressor.compression_method();

        let file_len = file_name.as_bytes().len();
        let general_purpose_flags: u16 = 1 << 11;

        let mut hasher = Hasher::new();
        let buffer: Vec<u8> = Vec::new();
        let mut async_writer = AsyncWriteWrapper::new(buffer);

        let total_read = compressor
            .compress(&mut async_writer, reader, &mut hasher)
            .await?;

        async_writer.flush().await?;
        let retreived_buffer = async_writer.retrieve_writer();
        let total_compress = retreived_buffer.len();
        let version_needed = compressor.version_needed();
        let crc = hasher.finalize();
        let extra_field_length = 0u16;

        let mut header = header![
            FILE_HEADER_BASE_SIZE + file_len;
            LOCAL_FILE_HEADER_SIGNATURE,          // Local file header signature.
            version_needed,                  // Version needed to extract.
            general_purpose_flags,    // General purpose flag (temporary crc and sizes + UTF-8 filename).
            compression_method,     // Compression method .
            time,                   // Modification time.
            date,                   // Modification date.
            crc,                   // Temporary CRC32.
            total_compress as u32,                   // Temporary compressed size.
            total_read as u32,                   // Temporary uncompressed size.
            file_len as u16,      // Filename length.
            extra_field_length,                   // Extra field length.
        ];
        header.extend_from_slice(file_name.as_bytes()); // Filename.
        self.sink.write_all(&header).await?;
        //self.sink.flush().await?;
        self.update_written_bytes_count(header.len() as u32);

        self.sink.write_all(&retreived_buffer).await?;
        self.update_written_bytes_count(total_compress as u32);

        self.files_info.push(ArchiveFileEntry {
            file_name_as_bytes: file_name.as_bytes().to_owned(),
            file_name_len: file_len as u16,
            compressed_size: total_compress as u32,
            uncompressed_size: total_read as u32,
            crc,
            offset,
            last_mod_file_time: time,
            last_mod_file_date: date,
            compressor,
            general_purpose_flags,
            extra_field_length,
            version_needed,
            compression_method,
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
                CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + file_info.file_name_len as usize;
                CENTRAL_DIRECTORY_ENTRY_SIGNATURE,                  // Central directory entry signature.
                file_info.version_made_by(),                      // Version made by.
                file_info.version_needed(),                          // Version needed to extract.
                file_info.general_purpose_flags,            // General purpose flag (temporary crc and sizes + UTF-8 filename).
                file_info.compressor.compression_method(),       // Compression method .
                file_info.last_mod_file_time,           // Modification time.
                file_info.last_mod_file_date,           // Modification date.
                file_info.crc,                  // CRC32.
                file_info.compressed_size,          // Compressed size.
                file_info.uncompressed_size,          // Uncompressed size.
                file_info.file_name_len,    // Filename length.
                0u16,                           // Extra field length.
                0u16,                           // File comment length.
                0u16,                           // File's Disk number.
                0u16,                           // Internal file attributes.
                (0o100644 << 16) as u32, // External file attributes (regular file / rw-r--r--).
                file_info.offset,        // Offset from start of file to local file header.
            ];
            entry.extend_from_slice(&file_info.file_name_as_bytes); // Filename.
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
            central_directory_offset: self.written_bytes_count,
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
