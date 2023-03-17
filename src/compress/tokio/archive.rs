use super::async_wrapper::{AsyncWriteSeekWrapper, AsyncWriteWrapper, CommonWrapper};
use super::compressor::compress;

use crate::archive::FileOptions;
use crate::archive_common::{
    build_central_directory_end, build_central_directory_file_header, build_file_header, set_sizes,
    ArchiveDescriptor, SubZipArchiveData, ZipArchiveCommon,
};
use crate::compression::Level;
use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, DATA_DESCRIPTOR_SIGNATURE, DESCRIPTOR_SIZE,
    EXTENDED_LOCAL_HEADER_FLAG, FILE_HEADER_CRC_OFFSET,
};
use crate::error::ArchiveError;

use crc32fast::Hasher;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use std::io::SeekFrom;

pub struct ZipArchive<'a, W: AsyncWrite + Unpin + 'a> {
    sink: Box<dyn CommonWrapper<W> + 'a>,
    data: SubZipArchiveData,
}

impl<'a, W: AsyncWrite + Unpin + 'a> ZipArchiveCommon for ZipArchive<'a, W> {
    fn get_archive_size(&self) -> u64 {
        self.data.archive_size
    }

    fn get_mut_data(&mut self) -> &mut SubZipArchiveData {
        &mut self.data
    }

    fn get_data(&self) -> &SubZipArchiveData {
        &self.data
    }
}

impl<'a, W: AsyncWrite + Unpin + Send + 'a> ZipArchive<'a, W> {
    /// Create a new zip archive, using the underlying `AsyncWrite` to write files' header and payload.
    pub fn new_streamable(sink_: W) -> Self {
        let mut data = SubZipArchiveData::default();
        data.base_flags = EXTENDED_LOCAL_HEADER_FLAG;
        Self {
            sink: Box::new(AsyncWriteWrapper::new(sink_)),
            data: SubZipArchiveData::default(),
        }
    }

    pub fn new<S: AsyncWrite + AsyncSeek + Unpin + 'a>(sink_: S) -> ZipArchive<'a, W>
    where
        AsyncWriteSeekWrapper<S>: CommonWrapper<W>,
    {
        let data = SubZipArchiveData::default();
        let wrapped_sink = AsyncWriteSeekWrapper::new(sink_);

        ZipArchive {
            sink: Box::new(wrapped_sink),
            data,
        }
    }

    pub fn get_archive_size(&mut self) -> u64 {
        self.sink.get_written_bytes_count().unwrap()
    }

    pub fn retrieve_writer(self) -> W {
        self.sink.get_into()
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
        reader: &mut R,
        options: &FileOptions<'a>,
    ) -> Result<(), ArchiveError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        let file_header_offset = self.data.archive_size;
        let mut hasher = Hasher::new();
        let compressor = options.compressor;

        let (file_header, mut archive_file_entry) = build_file_header(
            file_name,
            options,
            compressor,
            file_header_offset as u32,
            self.data.base_flags,
        );

        self.sink.write_all(file_header.buffer()).await?;

        let file_begin = self.sink.stream_position().await?;

        let uncompressed_size = compress(
            compressor,
            &mut self.sink,
            reader,
            &mut hasher,
            Level::Default,
        )
        .await?;

        let archive_size = self.sink.stream_position().await?;
        let compressed_size = archive_size - file_begin;

        let crc32 = hasher.finalize();
        archive_file_entry.crc32 = crc32;
        archive_file_entry.compressed_size = compressed_size;
        archive_file_entry.uncompressed_size = uncompressed_size;

        if self.data.base_flags & EXTENDED_LOCAL_HEADER_FLAG != 0 {
            let mut file_descriptor = ArchiveDescriptor::new(DESCRIPTOR_SIZE);
            file_descriptor.write_u32(DATA_DESCRIPTOR_SIGNATURE);
            set_sizes(
                &mut file_descriptor,
                crc32,
                compressed_size,
                uncompressed_size,
            );
        } else {
            let mut file_descriptor = ArchiveDescriptor::new(3 * 4);
            set_sizes(
                &mut file_descriptor,
                crc32,
                compressed_size,
                uncompressed_size,
            );

            //position in the the file header
            self.sink
                .seek(SeekFrom::Start(file_header_offset + FILE_HEADER_CRC_OFFSET))
                .await?;

            self.sink.write_all(file_descriptor.buffer()).await?;

            //position back at the end
            self.sink.seek(SeekFrom::Start(archive_size)).await?;
        }
        self.data.files_info.push(archive_file_entry);

        self.data.archive_size = self.sink.get_written_bytes_count()?;

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
    pub async fn finalize(mut self) -> Result<(u64, W), ArchiveError>
    where
        W: AsyncWrite + Unpin,
    {
        let central_directory_offset = self.sink.get_written_bytes_count()? as u32;

        let mut central_directory_header =
            ArchiveDescriptor::new(CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + 200);

        for file_info in &self.data.files_info {
            build_central_directory_file_header(&mut central_directory_header, file_info);

            self.sink
                .write_all(central_directory_header.buffer())
                .await?;
            central_directory_header.clear();
        }

        let current_archive_size = self.sink.get_written_bytes_count()?;
        let central_directory_size: u32 = current_archive_size as u32 - central_directory_offset;

        let end_of_central_directory = build_central_directory_end(
            &mut self.data,
            central_directory_offset,
            central_directory_size,
        );

        self.sink
            .write_all(end_of_central_directory.buffer())
            .await?;

        self.sink.flush().await?;

        self.data.archive_size = self.sink.get_written_bytes_count()?;

        Ok((self.data.archive_size, self.sink.get_into()))
    }

    pub fn set_archive_comment(&mut self, comment: &str) {
        self.data.set_archive_comment(comment);
    }
}
