use super::compressor::{self, compress};
use super::write_wrapper::{BytesCounter, WriteWrapper};

use crate::archive::FileOptions;
use crate::archive_common::{ArchiveDescriptor, SubZipArchiveData, ZipArchiveCommon};
use crate::compression::Level;
use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, DATA_DESCRIPTOR_SIGNATURE, DESCRIPTOR_SIZE,
    FILE_HEADER_CRC_OFFSET,
};
use crate::error::ArchiveError;

use crc32fast::Hasher;

use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct ZipArchive<W: Write> {
    sink: WriteWrapper<W>,
    data: SubZipArchiveData,
}

#[derive(Debug)]
pub struct ZipArchiveNoStream<W: Write + Seek> {
    sink: W,
    data: SubZipArchiveData,
    archive_size: u64,
}

impl<W: Write> ZipArchiveCommon for ZipArchive<W> {
    fn get_archive_size(&self) -> u64 {
        (self as &ZipArchive<W>).get_archive_size()
    }

    fn get_mut_data(&mut self) -> &mut SubZipArchiveData {
        &mut self.data
    }

    fn get_data(&self) -> &SubZipArchiveData {
        &self.data
    }
}

impl<W: Write> ZipArchive<W> {
    /// Create a new zip archive, using the underlying `Write` to write files' header and payload.
    pub fn new(sink_: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink: WriteWrapper::new(sink_),
            data: SubZipArchiveData::default(),
        }
    }

    pub fn get_archive_size(&self) -> u64 {
        self.sink.get_written_bytes_count()
    }

    pub fn retrieve_writer(self) -> W {
        self.sink.retrieve_writer()
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

    pub fn append_file<R>(
        &mut self,
        file_name: &str,
        reader: &mut R,
        options: &FileOptions,
    ) -> Result<(), ArchiveError>
    where
        W: Write,
        R: Read,
    {
        let compressor = options.compressor;

        let file_header_offset = self.sink.get_written_bytes_count();

        let (file_header, mut archive_file_entry) =
            self.build_file_header(file_name, options, compressor, file_header_offset as u32);

        self.sink.write_all(file_header.buffer())?;

        let mut hasher = Hasher::new();
        let cur_size = self.sink.get_written_bytes_count();

        let uncompressed_size = compressor::compress(
            compressor,
            &mut self.sink,
            reader,
            &mut hasher,
            options.compression_level,
        )?;

        let compressed_size = self.sink.get_written_bytes_count() - cur_size;
        let crc32 = hasher.finalize();

        archive_file_entry.crc32 = crc32;
        archive_file_entry.compressed_size = compressed_size as u32;
        archive_file_entry.uncompressed_size = uncompressed_size as u32;

        let mut file_descriptor = ArchiveDescriptor::new(DESCRIPTOR_SIZE);
        file_descriptor.write_u32(DATA_DESCRIPTOR_SIGNATURE);
        file_descriptor.write_u32(crc32);
        file_descriptor.write_u32(compressed_size as u32);
        file_descriptor.write_u32(uncompressed_size as u32);

        self.sink.write_all(file_descriptor.buffer())?;

        self.data.files_info.push(archive_file_entry);

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
    pub async fn finalize(&mut self) -> Result<(), ArchiveError>
    where
        W: Write,
    {
        let central_directory_offset = self.sink.get_written_bytes_count() as u32;

        let mut central_directory_header =
            ArchiveDescriptor::new(CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + 200);

        for file_info in &self.data.files_info {
            self.build_central_directory_file_header(&mut central_directory_header, file_info);

            self.sink.write_all(central_directory_header.buffer())?;

            central_directory_header.clear();
        }

        let current_archive_size = self.sink.get_written_bytes_count();
        let central_directory_size: u32 = current_archive_size as u32 - central_directory_offset;
        let end_of_central_directory =
            self.build_central_directory_end(central_directory_offset, central_directory_size);

        self.sink.write_all(end_of_central_directory.buffer())?;

        self.sink.flush()?;
        //println!("CentralDirectoryEnd {:#?}", dir_end);
        Ok(())
    }
}

impl<W: Write + Seek> ZipArchiveNoStream<W> {
    pub fn new(sink: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink,
            data: SubZipArchiveData::default(),
            archive_size: 0,
        }
    }

    pub async fn append_file<R>(
        &mut self,
        file_name: &str,
        reader: &mut R,
        options: &FileOptions,
    ) -> Result<(), ArchiveError>
    where
        W: Write + Seek,
        R: Read,
    {
        let file_header_offset = self.archive_size;
        let mut hasher = Hasher::new();
        let compressor = options.compressor;

        let (file_header, mut archive_file_entry) =
            self.build_file_header(file_name, options, compressor, file_header_offset as u32);

        self.sink.write_all(file_header.buffer())?;

        let file_begin = self.sink.stream_position()?;
        //println!("after header put: {file_begin} {file_begin:0X}");

        let uncompressed_size = compress(
            compressor,
            &mut self.sink,
            reader,
            &mut hasher,
            Level::Default,
        )? as u32;

        self.archive_size = self.sink.stream_position()?;
        let compressed_size = self.archive_size - file_begin;

        let crc32 = hasher.finalize();
        archive_file_entry.crc32 = crc32;
        archive_file_entry.compressed_size = compressed_size as u32;
        archive_file_entry.uncompressed_size = uncompressed_size;

        let mut file_data = ArchiveDescriptor::new(3 * 4);
        file_data.write_u32(crc32);
        file_data.write_u32(compressed_size as u32);
        file_data.write_u32(uncompressed_size);

        self.sink
            .seek(SeekFrom::Start(file_header_offset + FILE_HEADER_CRC_OFFSET))?;

        self.sink.write_all(file_data.buffer())?;

        self.sink.seek(SeekFrom::Start(self.archive_size))?;

        self.data.files_info.push(archive_file_entry);

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
    pub async fn finalize(&mut self) -> Result<(), ArchiveError>
    where
        W: Write,
    {
        let central_directory_offset = self.sink.stream_position()? as u32;

        let mut central_directory_header =
            ArchiveDescriptor::new(CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + 200);

        for file_info in &self.data.files_info {
            self.build_central_directory_file_header(&mut central_directory_header, file_info);

            self.sink.write_all(central_directory_header.buffer())?;

            central_directory_header.clear();
        }

        let current_archive_size = self.sink.stream_position()?;
        let central_directory_size: u32 = current_archive_size as u32 - central_directory_offset;

        let end_of_central_directory =
            self.build_central_directory_end(central_directory_offset, central_directory_size);

        self.sink.write_all(end_of_central_directory.buffer())?;

        self.sink.flush()?;
        self.archive_size = self.sink.stream_position()?;
        //println!("CentralDirectoryEnd {:#?}", dir_end);
        Ok(())
    }

    pub fn get_archive_size(&self) -> u64 {
        self.archive_size
    }
}

impl<W: Write + Seek> ZipArchiveCommon for ZipArchiveNoStream<W> {
    fn get_data(&self) -> &SubZipArchiveData {
        &self.data
    }

    fn get_mut_data(&mut self) -> &mut SubZipArchiveData {
        &mut self.data
    }

    fn get_archive_size(&self) -> u64 {
        (self as &ZipArchiveNoStream<W>).get_archive_size()
    }
}
