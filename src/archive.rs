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
        self.sink.get_written_bytes_count()
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

        self.sink.write_all(file_header.buffer()).await?;
        self.update_written_bytes_count(file_header.len() as u32);

        let mut hasher = Hasher::new();
        let cur_size = self.sink.get_written_bytes_count();

        let uncompressed_size = compressor
            .compress(&mut self.sink, reader, &mut hasher)
            .await?;

        //self.sink.flush().await?;
        let compressed_size = self.sink.get_written_bytes_count() - cur_size;
        self.update_written_bytes_count(compressed_size as u32);

        let crc32 = hasher.finalize();

        let mut file_descriptor = ArchiveDescriptor::new(DESCRIPTOR_SIZE);
        file_descriptor.write_u32(DATA_DESCRIPTOR_SIGNATURE);
        file_descriptor.write_u32(crc32);
        file_descriptor.write_u32(compressed_size as u32);
        file_descriptor.write_u32(uncompressed_size as u32);

        self.sink.write_all(file_descriptor.buffer()).await?;

        self.update_written_bytes_count(file_descriptor.len() as u32);

        self.files_info.push(ArchiveFileEntry {
            file_name_as_bytes: file_name_as_bytes_own,
            file_name_len: file_len,
            uncompressed_size: uncompressed_size as u32,
            compressed_size: compressed_size as u32,
            crc32,
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

        let file_nameas_bytes = file_name.as_bytes();
        let file_name_as_bytes_own = file_nameas_bytes.to_owned();
        let file_name_len = file_name_as_bytes_own.len() as u16;

        let mut general_purpose_flags: u16 = 0;
        if file_name_as_bytes_own.len() > file_name.len() {
            general_purpose_flags |= 1 << 11; //set utf8 flag
        }

        let mut hasher = Hasher::new();
        let buffer: Vec<u8> = Vec::new();
        let mut async_writer = AsyncWriteWrapper::new(buffer);

        let uncompressed_size = compressor
            .compress(&mut async_writer, reader, &mut hasher)
            .await? as u32;

        async_writer.flush().await?;
        let retreived_buffer = async_writer.retrieve_writer();
        let compressed_size = retreived_buffer.len() as u32;
        let version_needed = compressor.version_needed();
        let crc = hasher.finalize();
        let extra_field_length = 0u16;

        let mut file_header =
            ArchiveDescriptor::new(FILE_HEADER_BASE_SIZE + file_name_len as usize);
        file_header.write_u32(LOCAL_FILE_HEADER_SIGNATURE);
        file_header.write_u16(version_needed);
        file_header.write_u16(general_purpose_flags);
        file_header.write_u16(compression_method);
        file_header.write_u16(time);
        file_header.write_u16(date);
        file_header.write_u32(crc);
        file_header.write_u32(compressed_size);
        file_header.write_u32(uncompressed_size);
        file_header.write_u16(file_name_len);
        file_header.write_u16(extra_field_length);
        file_header.write_bytes(&file_name_as_bytes_own);

        self.sink.write_all(file_header.buffer()).await?;
        self.update_written_bytes_count(file_header.len() as u32);

        self.sink.write_all(&retreived_buffer).await?;
        self.update_written_bytes_count(compressed_size);

        self.files_info.push(ArchiveFileEntry {
            file_name_as_bytes: file_name.as_bytes().to_owned(),
            file_name_len,
            compressed_size,
            uncompressed_size,
            crc32: crc,
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
            let mut central_directory_header = ArchiveDescriptor::new(
                CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + file_info.file_name_len as usize,
            );

            central_directory_header.write_u32(CENTRAL_DIRECTORY_ENTRY_SIGNATURE); // Central directory entry signature.
            central_directory_header.write_u16(file_info.version_made_by()); // Version made by.
            central_directory_header.write_u16(file_info.version_needed()); // Version needed to extract.
            central_directory_header.write_u16(file_info.general_purpose_flags); // General purpose flag (temporary crc and sizes + UTF-8 filename).
            central_directory_header.write_u16(file_info.compression_method); // Compression method .
            central_directory_header.write_u16(file_info.last_mod_file_time); // Modification time.
            central_directory_header.write_u16(file_info.last_mod_file_date); // Modification date.
            central_directory_header.write_u32(file_info.crc32); // CRC32.
            central_directory_header.write_u32(file_info.compressed_size); // Compressed size.
            central_directory_header.write_u32(file_info.uncompressed_size); // Uncompressed size.
            central_directory_header.write_u16(file_info.file_name_len); // Filename length.
            central_directory_header.write_u16(0u16); // Extra field length.
            central_directory_header.write_u16(0u16); // File comment length.
            central_directory_header.write_u16(0u16); // File's Disk number.
            central_directory_header.write_u16(0u16); // Internal file attributes.
            central_directory_header.write_u32((0o100644 << 16) as u32); // External file attributes (regular file / rw-r--r--).
            central_directory_header.write_u32(file_info.offset); // Offset from start of file to local file header.
            central_directory_header.write_bytes(&file_info.file_name_as_bytes); // Filename.

            self.sink
                .write_all(central_directory_header.buffer())
                .await?;
            central_directory_size += central_directory_header.len();
        }

        let dir_end = CentralDirectoryEnd {
            disk_number: 0,
            disk_with_central_directory: 0,
            total_number_of_entries_on_this_disk: self.files_info.len() as u16,
            total_number_of_entries: self.files_info.len() as u16,
            central_directory_size: central_directory_size as u32,
            central_directory_offset: self.written_bytes_count,
            zip_file_comment_length: 0,
        };

        let mut end_of_central_directory = ArchiveDescriptor::new(END_OF_CENTRAL_DIRECTORY_SIZE);
        end_of_central_directory.write_u32(CENTRAL_DIRECTORY_END_SIGNATURE);
        end_of_central_directory.write_u16(dir_end.disk_number);
        end_of_central_directory.write_u16(dir_end.disk_with_central_directory);
        end_of_central_directory.write_u16(dir_end.total_number_of_entries_on_this_disk);
        end_of_central_directory.write_u16(dir_end.total_number_of_entries);
        end_of_central_directory.write_u32(dir_end.central_directory_size);
        end_of_central_directory.write_u32(dir_end.central_directory_offset);
        end_of_central_directory.write_u16(dir_end.zip_file_comment_length);

        self.sink
            .write_all(end_of_central_directory.buffer())
            .await?;

        //println!("CentralDirectoryEnd {:#?}", dir_end);
        Ok(())
    }
}

#[derive(Debug)]
pub struct CentralDirectoryEnd {
    pub disk_number: u16,
    pub disk_with_central_directory: u16,
    pub total_number_of_entries_on_this_disk: u16,
    pub total_number_of_entries: u16,
    pub central_directory_size: u32,
    pub central_directory_offset: u32,
    pub zip_file_comment_length: u16,
}
