use super::compressor::{self, compress};
use super::write_wrapper::{BytesCounter, WriteSeekWrapper, WriteWrapper};

use crate::archive::FileOptions;
use crate::archive_common::{
    build_central_directory_end, build_central_directory_file_header, ArchiveDescriptor,
    SubZipArchiveData, ZipArchiveCommon,
};
use crate::compression::Level;
use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, DATA_DESCRIPTOR_SIGNATURE, DESCRIPTOR_SIZE,
    FILE_HEADER_CRC_OFFSET,
};
use crate::error::ArchiveError;
use crate::types::ArchiveFileEntry;

use crc32fast::Hasher;

use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct ZipArchive<W: Write> {
    sink: WriteWrapper<W>,
    data: SubZipArchiveData,
    archive_size: u64,
}

#[derive(Debug)]
pub struct ZipArchiveNoStream<W: Write + Seek> {
    sink: WriteSeekWrapper<W>,
    data: SubZipArchiveData,
    archive_size: u64,
}

impl<W: Write> ZipArchiveCommon for ZipArchive<W> {
    fn get_archive_size(&self) -> u64 {
        self.archive_size
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
            archive_size: 0,
        }
    }

    pub fn get_archive_size(&mut self) -> Result<u64, ArchiveError> {
        Ok(self.sink.get_written_bytes_count()?)
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

        let file_header_offset = self.archive_size;

        let (file_header, mut archive_file_entry) = self.build_file_header(
            file_name,
            options,
            compressor,
            file_header_offset as u32,
            true,
        );

        self.sink.write_all(file_header.buffer())?;

        let mut hasher = Hasher::new();
        let cur_size = self.sink.get_written_bytes_count()?;

        let uncompressed_size = compressor::compress(
            compressor,
            &mut self.sink,
            reader,
            &mut hasher,
            options.compression_level,
        )?;

        let stream_position = self.sink.get_written_bytes_count()?;
        let compressed_size = stream_position - cur_size;
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

        println!("{}", archive_file_entry);
        self.data.files_info.push(archive_file_entry);
        self.archive_size = self.sink.get_written_bytes_count()?;
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
    pub fn finalize(&mut self) -> Result<(), ArchiveError>
    where
        W: Write,
    {
        finalize_std_comon(&mut self.sink, &self.data, &self.data.files_info)?;

        Ok(())
    }
}

impl<W: Write + Seek> ZipArchiveNoStream<W> {
    pub fn new(sink: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink: WriteSeekWrapper::new(sink),
            data: SubZipArchiveData::default(),
            archive_size: 0,
        }
    }

    pub fn append_file<R>(
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

        let (file_header, mut archive_file_entry) = self.build_file_header(
            file_name,
            options,
            compressor,
            file_header_offset as u32,
            false,
        );

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

        let archive_size = self.sink.stream_position()?;
        let compressed_size = archive_size - file_begin;

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

        self.sink.seek(SeekFrom::Start(archive_size))?;

        self.data.files_info.push(archive_file_entry);

        self.archive_size = self.sink.get_written_bytes_count()?;
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
    pub fn finalize(&mut self) -> Result<(), ArchiveError>
    where
        W: Write,
    {
        self.archive_size = finalize_std_comon(&mut self.sink, &self.data, &self.data.files_info)?;

        Ok(())
    }

    pub fn get_archive_size(&mut self) -> Result<u64, ArchiveError> {
        Ok(self.sink.get_written_bytes_count()?)
    }
}

fn finalize_std_comon<T>(
    sink: &mut T,
    data: &SubZipArchiveData,
    files_info: &Vec<ArchiveFileEntry>,
) -> Result<u64, ArchiveError>
where
    T: BytesCounter + Write,
{
    let central_directory_offset = sink.get_written_bytes_count()? as u32;

    let mut central_directory_header =
        ArchiveDescriptor::new(CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + 200);

    for file_info in files_info {
        build_central_directory_file_header(&mut central_directory_header, file_info);

        sink.write_all(central_directory_header.buffer())?;
        central_directory_header.clear();
    }

    let current_archive_size = sink.get_written_bytes_count()?;
    let central_directory_size: u32 = current_archive_size as u32 - central_directory_offset;

    let end_of_central_directory =
        build_central_directory_end(data, central_directory_offset, central_directory_size);

    sink.write_all(end_of_central_directory.buffer())?;

    sink.flush()?;

    let archive_size = sink.get_written_bytes_count()?;

    Ok(archive_size)
}

impl<W: Write + Seek> ZipArchiveCommon for ZipArchiveNoStream<W> {
    fn get_data(&self) -> &SubZipArchiveData {
        &self.data
    }

    fn get_mut_data(&mut self) -> &mut SubZipArchiveData {
        &mut self.data
    }

    fn get_archive_size(&self) -> u64 {
        self.archive_size
    }
}
