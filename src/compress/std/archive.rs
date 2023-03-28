use super::compressor::compress;
use super::write_wrapper::{CommonWrapper, WriteSeekWrapper, WriteWrapper};

use crate::archive_common::{
    build_central_directory_end, build_central_directory_file_header, build_data_descriptor,
    build_file_header, ArchiveDescriptor, SubZipArchiveData, ZipArchiveCommon,
};
use crate::compress::FileOptions;
use crate::compression::Level;
use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, EXTENDED_LOCAL_HEADER_FLAG, FILE_HEADER_CRC_OFFSET,
};
use crate::error::ArchiveError;
use crc32fast::Hasher;
use std::io::{Read, Seek, SeekFrom, Write};

/// A zip archive.
///
/// Create a zip archive using either:
/// * [`new_streamable`](Self::new_streamable()), or
/// * [`new`](Self::new()).
///
/// Then, append files one by one using the [`append`](Self::append()) function.
/// When finished, use the [`finalize`](Self::finalize()) function.
///
/// # Features
///
/// Requires `std` feature
pub struct ZipArchive<'a, W: Write> {
    sink: Box<dyn CommonWrapper<W> + 'a>,
    data: SubZipArchiveData,
}

impl<'a, W: Write> ZipArchiveCommon for ZipArchive<'a, W> {
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

impl<'a, W: Write + 'a> ZipArchive<'a, W> {
    /// Create a new zip archive, using the underlying [`Write`] to write
    /// files' header and payload.
    ///
    pub fn new_streamable(sink: W) -> Self {
        let mut data = SubZipArchiveData::default();
        data.base_flags = EXTENDED_LOCAL_HEADER_FLAG; //extended local header
        Self {
            sink: Box::new(WriteWrapper::new(sink)),
            data,
        }
    }

    /// Create a new zip archive (non streamable), using the underlying [`Write`] + [`Seek`] to
    /// write files' header and payload.
    ///
    /// _Note:_ a non streamable archive save few bytes per files.
    pub fn new<S: Write + Seek + 'a>(sink: S) -> ZipArchive<'a, W>
    where
        WriteSeekWrapper<S>: CommonWrapper<W>,
    {
        let data = SubZipArchiveData::default();
        let wrapped_sink = WriteSeekWrapper::new(sink);

        ZipArchive {
            sink: Box::new(wrapped_sink),
            data,
        }
    }

    /// Get archive current total bytes written.
    pub fn get_archive_size(&mut self) -> Result<u64, ArchiveError> {
        Ok(self.sink.get_written_bytes_count()?)
    }

    /// Append a new entity to the archive using the provided name, options and payload as [`Read`] object to
    /// be compress.  
    ///
    /// # Arguments
    /// * `file_name` - A for the name of the archive entry
    /// * `reader` -  The entity's payload as a [`Read`]
    /// * `options` - Entry's archive options
    ///
    pub fn append<R>(
        &mut self,
        file_name: &str,
        options: &FileOptions,
        payload: &mut R,
    ) -> Result<(), ArchiveError>
    where
        W: Write,
        R: Read,
    {
        let file_header_offset = self.data.archive_size;
        let mut hasher = Hasher::new();
        let compressor = options.compression_method;

        let (file_header, mut archive_file_entry) = build_file_header(
            file_name,
            options,
            compressor,
            file_header_offset,
            &self.data,
        );

        self.sink.write_all(file_header.buffer())?;

        let file_begin = self.sink.stream_position()?;

        let uncompressed_size = compress(
            compressor,
            &mut self.sink,
            payload,
            &mut hasher,
            Level::Default,
        )?;

        let archive_size = self.sink.stream_position()?;
        let compressed_size = archive_size - file_begin;

        let crc32 = hasher.finalize();
        archive_file_entry.crc32 = crc32;
        archive_file_entry.compressed_size = compressed_size;
        archive_file_entry.uncompressed_size = uncompressed_size;

        if self.data.base_flags & EXTENDED_LOCAL_HEADER_FLAG != 0 {
            let data_descriptor = build_data_descriptor(crc32, compressed_size, uncompressed_size);
            self.sink.write_all(data_descriptor.buffer())?;
        } else {
            let mut file_descriptor = ArchiveDescriptor::new(3 * 4);

            file_descriptor.write_u32(crc32);
            file_descriptor.write_u32(compressed_size as u32);
            file_descriptor.write_u32(uncompressed_size as u32);

            //position in the the file header
            self.sink
                .seek(SeekFrom::Start(file_header_offset + FILE_HEADER_CRC_OFFSET))?;

            self.sink.write_all(file_descriptor.buffer())?;

            //position back at the end
            self.sink.seek(SeekFrom::Start(archive_size))?;
        }
        self.data.add_archive_file_entry(archive_file_entry);

        self.data.archive_size = self.sink.get_written_bytes_count()?;

        Ok(())
    }

    /// Finalize the archive by writing the necessary metadata to the end of the archive.
    ///
    /// Returns the archive size (bytes) and the [Write] object passed at creation.
    pub fn finalize(mut self) -> Result<(u64, W), ArchiveError>
    where
        W: Write,
    {
        let central_directory_offset = self.sink.get_written_bytes_count()?;

        let mut central_directory_header =
            ArchiveDescriptor::new(CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + 200);

        for file_info in self.data.iter() {
            build_central_directory_file_header(&mut central_directory_header, file_info);

            self.sink.write_all(central_directory_header.buffer())?;
            central_directory_header.clear();
        }

        let current_archive_size = self.sink.get_written_bytes_count()?;
        let central_directory_size: u64 = current_archive_size - central_directory_offset;

        let end_of_central_directory = build_central_directory_end(
            &mut self.data,
            central_directory_offset,
            central_directory_size,
        );

        self.sink.write_all(end_of_central_directory.buffer())?;

        self.sink.flush()?;

        self.data.archive_size = self.sink.get_written_bytes_count()?;

        Ok((self.data.archive_size, self.sink.get_into()))
    }

    ///Set the archive comment
    pub fn set_archive_comment(&mut self, comment: &str) {
        self.data.set_archive_comment(comment);
    }
}
