use super::compressor::compress;
use super::write_wrapper::{CommonWrapper, WriteSeekWrapper, WriteWrapper};

use crate::archive_common::{ArchiveDescriptor, ExtraFieldZIP64ExtendedInformation};
use crate::compress::common::{
    build_central_directory_end, build_central_directory_file_header, build_data_descriptor,
    build_file_header, build_file_sizes_update, is_streaming, SubZipArchiveData, ZipArchiveCommon,
};
use crate::compress::FileOptions;
use crate::compression::{CompressionMethod, Level};
use crate::constants::{EXTENDED_LOCAL_HEADER_FLAG, FILE_HEADER_BASE_SIZE, FILE_HEADER_CRC_OFFSET};
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

        let (file_header, mut archive_file_entry, zip_extra_offset) = build_file_header(
            file_name,
            options,
            compressor,
            file_header_offset,
            &self.data,
            false,
        );

        self.sink.write_all(file_header.buffer())?;

        let file_begin = self.sink.stream_position()?;

        let (uncompressed_size, is_text) = compress(
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
        archive_file_entry.apparently_text_file(is_text);

        if is_streaming(archive_file_entry.general_purpose_flags) {
            let data_descriptor = build_data_descriptor(&archive_file_entry);
            self.sink.write_all(data_descriptor.buffer())?;
        } else {
            let sizes_update = build_file_sizes_update(&archive_file_entry);

            //position in the the file header
            self.sink
                .seek(SeekFrom::Start(file_header_offset + FILE_HEADER_CRC_OFFSET))?;

            self.sink.write_all(sizes_update.buffer())?;

            //position back at the end
            self.sink.seek(SeekFrom::Start(archive_size))?;

            if archive_file_entry.is_zip64() {
                if options.large_file {
                    if let Some(zip64_extra_field) = archive_file_entry.extra_fields.last() {
                        let mut file_descriptor = ArchiveDescriptor::new(30);
                        zip64_extra_field
                            .file_header_write_data(&mut file_descriptor, &archive_file_entry);

                        self.sink.seek(SeekFrom::Start(
                            file_header_offset + FILE_HEADER_BASE_SIZE + zip_extra_offset,
                        ))?;

                        self.sink.write_all(file_descriptor.buffer())?;
                        //position back at the end
                        self.sink.seek(SeekFrom::Start(archive_size))?;
                    }
                }
            } else {
                //it wasn't identified as zip64 from option, but it can be as stream
                let data_descriptor = build_data_descriptor(&archive_file_entry);
                self.sink.write_all(data_descriptor.buffer())?;
            }
        }

        if !archive_file_entry.has_zip64_extra_field && archive_file_entry.is_zip64() {
            let zip_extra_field = ExtraFieldZIP64ExtendedInformation::new();
            archive_file_entry
                .extra_fields
                .push(Box::new(zip_extra_field));
        }

        self.data.add_archive_file_entry(archive_file_entry);

        self.data.archive_size = self.sink.get_written_bytes_count()?;

        Ok(())
    }

    /// Append a directory entry to the archive.
    ///
    ///
    pub fn append_directory(
        &mut self,
        file_name: &str,
        options: &FileOptions<'a>,
    ) -> Result<(), ArchiveError>
    where
        W: Write,
    {
        let file_header_offset = self.data.archive_size;
        let compressor = CompressionMethod::Store();

        //ensure that the name end with a slash ('/')
        let new_file_name = match file_name.chars().last() {
            Some('/') | Some('\\') => file_name.to_owned(),
            _ => {
                let mut s = file_name.to_owned();
                s.push('/');
                s
            }
        };

        let (file_header, mut archive_file_entry, _zip_extra_offset) = build_file_header(
            &new_file_name,
            options,
            compressor,
            file_header_offset,
            &self.data,
            true,
        );
        archive_file_entry.general_purpose_flags &= !EXTENDED_LOCAL_HEADER_FLAG;

        self.sink.write_all(file_header.buffer())?;

        let (uncompressed_size, is_text) = (0, false);

        let compressed_size = 0;

        archive_file_entry.crc32 = 0;
        archive_file_entry.compressed_size = compressed_size;
        archive_file_entry.uncompressed_size = uncompressed_size;
        archive_file_entry.apparently_text_file(is_text);

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
        println!(
            "central_directory_offset  {:?}  {:0X}",
            central_directory_offset, central_directory_offset
        );
        let mut central_directory_header = ArchiveDescriptor::new(500);

        for file_info in self.data.iter() {
            let off = self.sink.get_written_bytes_count()?;
            println!("FILE OFFSET  {:?}  {:0X}", off, off);

            build_central_directory_file_header(&mut central_directory_header, file_info);

            self.sink.write_all(central_directory_header.buffer())?;
            central_directory_header.clear();
        }

        let current_archive_size = self.sink.get_written_bytes_count()?;
        let central_directory_size = current_archive_size - central_directory_offset;

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
