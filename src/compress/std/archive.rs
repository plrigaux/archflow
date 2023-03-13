use super::compressor::compress;
use super::write_wrapper::{CommonWrapper, WriteSeekWrapper, WriteWrapper};

use crate::archive::FileOptions;
use crate::archive_common::{
    build_central_directory_end, build_central_directory_file_header, build_file_header,
    ArchiveDescriptor, SubZipArchiveData, ZipArchiveCommon,
};
use crate::compression::Level;
use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, DATA_DESCRIPTOR_SIGNATURE, DESCRIPTOR_SIZE,
    FILE_HEADER_CRC_OFFSET,
};
use crate::error::ArchiveError;
use crc32fast::Hasher;
use std::io::{Read, Seek, SeekFrom, Write};

pub struct ZipArchive<W: Write> {
    sink: WriteWrapper<W>,
    data: SubZipArchiveData,
}

#[derive(Debug)]
pub struct ZipArchiveNoStream<W: Write + Seek> {
    sink: WriteSeekWrapper<W>,
    data: SubZipArchiveData,
}

impl<W: Write> ZipArchiveCommon for ZipArchive<W> {
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

impl<W: Write> ZipArchive<W> {
    /// Create a new zip archive, using the underlying `Write` to write files' header and payload.
    pub fn new(sink_: W) -> Self {
        let mut data = SubZipArchiveData::default();
        data.data_descriptor = true;
        Self {
            sink: WriteWrapper::new(sink_),
            data,
        }
    }

    pub fn get_archive_size(&mut self) -> Result<u64, ArchiveError> {
        Ok(self.sink.get_written_bytes_count()?)
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
        append_file_std_common(&mut self.sink, &mut self.data, file_name, reader, options)
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
    pub fn finalize(mut self) -> Result<(u64, W), ArchiveError>
    where
        W: Write,
    {
        self.data.archive_size = finalize_std_comon(&mut self.sink, &self.data)?;

        Ok((self.data.archive_size, self.sink.get_into()))
    }
}

impl<W: Write + Seek> ZipArchiveNoStream<W> {
    pub fn new(sink: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink: WriteSeekWrapper::new(sink),
            data: SubZipArchiveData::default(),
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
        append_file_std_common(&mut self.sink, &mut self.data, file_name, reader, options)
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
    pub fn finalize(mut self) -> Result<(u64, W), ArchiveError>
    where
        W: Write,
    {
        self.data.archive_size = finalize_std_comon(&mut self.sink, &self.data)?;

        Ok((self.data.archive_size, self.sink.get_into()))
    }

    pub fn get_archive_size(&mut self) -> Result<u64, ArchiveError> {
        Ok(self.sink.get_written_bytes_count()?)
    }
}

fn append_file_std_common<W, R, T>(
    sink: &mut T,
    data: &mut SubZipArchiveData,
    file_name: &str,
    reader: &mut R,
    options: &FileOptions,
) -> Result<(), ArchiveError>
where
    T: CommonWrapper<W>,
    W: Write,
    R: Read,
{
    let file_header_offset = data.archive_size;
    let mut hasher = Hasher::new();
    let compressor = options.compressor;

    let (file_header, mut archive_file_entry) = build_file_header(
        file_name,
        options,
        compressor,
        file_header_offset as u32,
        data.data_descriptor,
    );

    sink.write_all(file_header.buffer())?;

    let file_begin = sink.stream_position()?;

    let uncompressed_size = compress(compressor, sink, reader, &mut hasher, Level::Default)?;

    let archive_size = sink.stream_position()?;
    let compressed_size = archive_size - file_begin;

    let crc32 = hasher.finalize();
    archive_file_entry.crc32 = crc32;
    archive_file_entry.compressed_size = compressed_size;
    archive_file_entry.uncompressed_size = uncompressed_size;

    if data.data_descriptor {
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
        sink.seek(SeekFrom::Start(file_header_offset + FILE_HEADER_CRC_OFFSET))?;

        sink.write_all(file_descriptor.buffer())?;

        //position back at the end
        sink.seek(SeekFrom::Start(archive_size))?;
    }
    data.files_info.push(archive_file_entry);

    data.archive_size = sink.get_written_bytes_count()?;

    Ok(())
}

fn set_sizes(
    file_descriptor: &mut ArchiveDescriptor,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
) {
    file_descriptor.write_u32(crc32);
    file_descriptor.write_u32(compressed_size as u32);
    file_descriptor.write_u32(uncompressed_size as u32);
}

fn finalize_std_comon<T, W>(sink: &mut T, data: &SubZipArchiveData) -> Result<u64, ArchiveError>
where
    T: CommonWrapper<W>,
    W: Write,
{
    let central_directory_offset = sink.get_written_bytes_count()? as u32;

    let mut central_directory_header =
        ArchiveDescriptor::new(CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + 200);

    for file_info in &data.files_info {
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
        self.data.archive_size
    }
}
