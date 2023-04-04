use std::fmt::Debug;
use std::str;
use std::u32;
use std::u8;

use super::compression::CompressionMethod;

use crate::compress::FileOptions;
use crate::constants::CENTRAL_DIRECTORY_END_SIGNATURE;
use crate::constants::CENTRAL_DIRECTORY_ENTRY_SIGNATURE;
use crate::constants::DATA_DESCRIPTOR_SIGNATURE;
use crate::constants::EXTENDED_LOCAL_HEADER_FLAG;
use crate::constants::FILE_HEADER_BASE_SIZE;
use crate::constants::LOCAL_FILE_HEADER_SIGNATURE;
use crate::constants::VERSION_MADE_BY;
use crate::constants::X5455_EXTENDEDTIMESTAMP;
use crate::constants::ZIP64_CENTRAL_DIRECTORY_END_SIGNATURE;
use crate::constants::ZIP64_DESCRIPTOR_SIZE;
use crate::constants::ZIP64_END_OF_CENTRAL_DIR_LOCATOR_SIGNATURE;
use crate::error::ArchiveError;
use crate::types::ArchiveFileEntry;

pub trait ZipArchiveCommon {
    fn get_archive_size(&self) -> u64;
    fn get_data(&self) -> &SubZipArchiveData;
    fn get_mut_data(&mut self) -> &mut SubZipArchiveData;

    fn set_archive_comment(&mut self, comment: &str) {
        self.get_mut_data().set_archive_comment(comment);
    }
}

pub fn build_file_header(
    file_name: &str,
    options: &FileOptions,
    compressor: CompressionMethod,
    offset: u64,
    data: &SubZipArchiveData,
) -> (ArchiveDescriptor, ArchiveFileEntry, u64) {
    let file_nameas_bytes = file_name.as_bytes();
    let file_name_as_bytes_own = file_nameas_bytes.to_owned();
    let file_name_len = file_name_as_bytes_own.len() as u16;

    let (date, time) = options.last_modified_time.ms_dos();
    let mut general_purpose_flags: u16 = data.base_flags;
    if file_name_as_bytes_own.len() > file_name.len() {
        general_purpose_flags |= 1 << 11; //set utf8 flag
    }

    let file_comment = if let Some(comment) = options.comment {
        let file_comment_as_bytes_own = comment.as_bytes().to_owned();
        if file_comment_as_bytes_own.len() > comment.len() {
            general_purpose_flags |= 1 << 11; //set utf8 flag
        }
        Some(file_comment_as_bytes_own)
    } else {
        None
    };

    general_purpose_flags = compressor
        .update_general_purpose_bit_flag(general_purpose_flags, options.compression_level);

    let minimum_version_needed_to_extract = compressor.zip_version_needed();
    let version_made_by = options.system.update_version_needed(VERSION_MADE_BY);

    let mut extra_fields: Vec<Box<dyn ExtraFields>> = Vec::new();

    if options.last_modified_time.extended_timestamp() {
        let ts = ExtraFieldExtendedTimestamp::new(
            Some(options.last_modified_time.timestamp()),
            None,
            None,
        );
        extra_fields.push(Box::new(ts));
    }

    let mut zip64_extra_field_added = false;
    if options.large_file && !is_streaming(data.base_flags) {
        //Be sure to make it last
        let ts = ExtraFieldZIP64ExtendedInformation::new();
        extra_fields.push(Box::new(ts));
        zip64_extra_field_added = true;
    }

    let mut archive_file_entry = ArchiveFileEntry {
        version_made_by,
        minimum_version_needed_to_extract,
        general_purpose_flags,
        compression_method: compressor.zip_code(),
        last_mod_file_time: time,
        last_mod_file_date: date,
        crc32: 0,
        compressed_size: 0,
        uncompressed_size: 0,
        file_name_len,
        extra_field_length: 0,
        file_name_as_bytes: file_name.as_bytes().to_owned(),
        offset,
        compressor,
        internal_file_attributes: 0,
        external_file_attributes: 0,
        file_disk_number: 0,
        extra_fields,
        file_comment,
        has_zip64_extra_field: zip64_extra_field_added,
    };

    let mut extended_data_buffer = ArchiveDescriptor::new(500);

    let mut zip_extra_offset: u64 = 0;
    let mut it = archive_file_entry.extra_fields.iter().peekable();
    while let Some(extra_field) = it.next() {
        if it.peek().is_none() {
            zip_extra_offset = extended_data_buffer.len() as u64;
        }
        extra_field.file_header_write_data(&mut extended_data_buffer, &archive_file_entry);
    }

    archive_file_entry.extra_field_length = extended_data_buffer.len() as u16;

    let mut file_header = ArchiveDescriptor::new(FILE_HEADER_BASE_SIZE + file_name_len as u64);
    file_header.write_u32(LOCAL_FILE_HEADER_SIGNATURE);
    file_header.write_u16(minimum_version_needed_to_extract);
    file_header.write_u16(general_purpose_flags);
    file_header.write_u16(archive_file_entry.compression_method);
    file_header.write_u16(time);
    file_header.write_u16(date);
    file_header.write_u32(0); // CRC-32
    file_header.write_u32(0); // compressed size
    file_header.write_u32(0); // uncompressed size
    file_header.write_u16(file_name_len); // file name length
    file_header.write_u16(archive_file_entry.extra_field_length); //extra field length
    file_header.write_bytes(&file_name_as_bytes_own);
    file_header.write_bytes(extended_data_buffer.bytes());

    (file_header, archive_file_entry, zip_extra_offset)
}

pub fn build_central_directory_file_header(
    central_directory_header: &mut ArchiveDescriptor,
    file_info: &mut ArchiveFileEntry,
) {
    let mut extra_field_buffer = ArchiveDescriptor::new(file_info.extra_field_length as u64);

    for extra_field in &file_info.extra_fields {
        extra_field.central_header_extra_write_data(&mut extra_field_buffer, file_info)
    }

    file_info.extra_field_length = extra_field_buffer.len() as u16;

    central_directory_header.write_u32(CENTRAL_DIRECTORY_ENTRY_SIGNATURE); // Central directory entry signature.
    central_directory_header.write_u16(file_info.version_made_by); // Version made by.
    central_directory_header.write_u16(file_info.version_needed_to_extract()); // Version needed to extract.
    central_directory_header.write_u16(file_info.general_purpose_flags); // General purpose flag (temporary crc and sizes + UTF-8 filename).
    central_directory_header.write_u16(file_info.compression_method); // Compression method .
    central_directory_header.write_u16(file_info.last_mod_file_time); // Modification time.
    central_directory_header.write_u16(file_info.last_mod_file_date); // Modification date.
    central_directory_header.write_u32(file_info.crc32); // CRC32.
    central_directory_header.write_u32(file_info.zip64_compressed_size()); // Compressed size.
    central_directory_header.write_u32(file_info.zip64_uncompressed_size()); // Uncompressed size.
    central_directory_header.write_u16(file_info.file_name_len); // Filename length.
    central_directory_header.write_u16(file_info.extra_field_length); // Extra field length.
    central_directory_header.write_u16(file_info.file_comment_length()); // File comment length.
    central_directory_header.write_u16(file_info.file_disk_number as u16); // File's Disk number.
    central_directory_header.write_u16(file_info.internal_file_attributes); // Internal file attributes.
    central_directory_header.write_u32((0o100644 << 16) as u32); // External file attributes (regular file / rw-r--r--).
    central_directory_header.write_u32(file_info.zip64_offset()); // Offset from start of file to local file header.
    central_directory_header.write_bytes(&file_info.file_name_as_bytes); // Filename.

    central_directory_header.write_bytes(extra_field_buffer.bytes()); // file comment.

    if let Some(comment) = &file_info.file_comment {
        central_directory_header.write_bytes(comment); // file comment.
    }
}

pub fn build_data_descriptor(archive_file_entry: &ArchiveFileEntry) -> ArchiveDescriptor {
    let mut file_descriptor = ArchiveDescriptor::new(ZIP64_DESCRIPTOR_SIZE);
    file_descriptor.write_u32(DATA_DESCRIPTOR_SIGNATURE); //This is optional
    file_descriptor.write_u32(archive_file_entry.crc32);

    if archive_file_entry.is_zip64() {
        file_descriptor.write_u64(archive_file_entry.compressed_size);
        file_descriptor.write_u64(archive_file_entry.uncompressed_size);
    } else {
        file_descriptor.write_u32(archive_file_entry.compressed_size as u32);
        file_descriptor.write_u32(archive_file_entry.uncompressed_size as u32);
    }

    file_descriptor
}

pub fn build_file_sizes_update(archive_file_entry: &ArchiveFileEntry) -> ArchiveDescriptor {
    let mut file_descriptor = ArchiveDescriptor::new(3 * 4);

    file_descriptor.write_u32(archive_file_entry.crc32);
    file_descriptor.write_u32(archive_file_entry.zip64_compressed_size());
    file_descriptor.write_u32(archive_file_entry.zip64_uncompressed_size());

    file_descriptor
}

pub fn build_central_directory_end(
    data: &mut SubZipArchiveData,
    central_directory_offset: u64,
    central_directory_size: u64,
) -> ArchiveDescriptor {
    data.central_directory_end.number_of_this_disk = 0;
    data.central_directory_end
        .number_of_the_disk_with_central_directory = 0;
    data.central_directory_end
        .total_number_of_entries_on_this_disk = data.files_info.len() as u64;
    data.central_directory_end
        .total_number_of_entries_in_the_central_directory = data.files_info.len() as u64;
    data.central_directory_end.central_directory_size = central_directory_size;
    data.central_directory_end
        .offset_of_start_of_central_directory = central_directory_offset;

    let mut end_of_central_directory = ArchiveDescriptor::new(500); //TODO calculate capacity size

    if data.central_directory_end.needs_zip64_format_extensions() {
        //[zip64 end of central directory record]

        data.central_directory_end
            .create_zip64_end_of_central_directory_record(&mut end_of_central_directory);
        //------------------------------------------------
        //[zip64 end of central directory locator]
        //------------------------------------------------
        data.central_directory_end
            .create_end_of_central_directory_locator(&mut end_of_central_directory);
    }

    //4.4.1.5  The end of central directory record and the Zip64 end
    //of central directory locator record MUST reside on the same
    //disk when splitting or spanning an archive.
    data.central_directory_end
        .create_end_of_central_directory(&mut end_of_central_directory);

    end_of_central_directory
}

#[derive(Debug, Default)]
pub struct SubZipArchiveData {
    files_info: Vec<ArchiveFileEntry>,
    central_directory_end: CentralDirectoryEnd,
    pub archive_size: u64,
    pub base_flags: u16,
    is_big_archive: bool,
}

impl SubZipArchiveData {
    pub fn set_archive_comment(&mut self, comment: &str) {
        self.central_directory_end.set_archive_comment(comment)
    }

    pub fn add_archive_file_entry(&mut self, archive_file_entry: ArchiveFileEntry) {
        self.is_big_archive |= archive_file_entry.is_zip64();
        self.files_info.push(archive_file_entry)
    }

    pub fn iter(&mut self) -> std::slice::IterMut<'_, ArchiveFileEntry> {
        self.files_info.iter_mut()
    }
}

#[derive(Debug)]
pub struct ArchiveDescriptor {
    buffer: Vec<u8>,
}

impl ArchiveDescriptor {
    pub fn new(capacity: u64) -> ArchiveDescriptor {
        ArchiveDescriptor {
            buffer: Vec::with_capacity(capacity as usize),
        }
    }

    pub fn write_u8(&mut self, val: u8) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_u16(&mut self, val: u16) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_u32(&mut self, val: u32) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_i32(&mut self, val: i32) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_u64(&mut self, val: u64) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_str(&mut self, val: &str) {
        self.write_bytes(val.as_bytes());
    }

    pub fn write_bytes(&mut self, val: &[u8]) {
        self.buffer.extend_from_slice(val);
    }

    pub fn write_bytes_len(&mut self, val: &[u8], max_len: usize) {
        self.buffer.extend(val.iter().take(max_len));
    }

    pub fn write_zeros(&mut self, len: usize) {
        self.buffer.resize(self.len() + len, 0);
    }

    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn read_file_descriptor(stream: &[u8]) -> Result<ArchiveFileEntry, ArchiveError> {
        let mut indexer = ArchiveDescriptorReader::new();

        let _signature = indexer.read_u32(stream);
        let version_needed = indexer.read_u16(stream) & 0xFF;
        let general_purpose_flags = indexer.read_u16(stream);
        let compression_method = indexer.read_u16(stream);
        let time = indexer.read_u16(stream);
        let date = indexer.read_u16(stream);
        let crc = indexer.read_u32(stream);
        let compressed_size = indexer.read_u32(stream) as u64;
        let uncompressed_size = indexer.read_u32(stream) as u64;
        let file_name_len = indexer.read_u16(stream);
        let extra_field_length = indexer.read_u16(stream);
        let file_name = indexer.read_utf8_string(stream, file_name_len as usize);

        let file_name_as_bytes = file_name.as_bytes().to_owned();

        let archive_file_entry = ArchiveFileEntry {
            version_made_by: 0,
            minimum_version_needed_to_extract: version_needed,
            general_purpose_flags,
            last_mod_file_time: time,
            last_mod_file_date: date,
            crc32: crc,
            compressed_size,
            uncompressed_size,
            file_name_len,
            extra_field_length,
            file_name_as_bytes,
            offset: 0,
            internal_file_attributes: 0,
            external_file_attributes: 0,
            file_disk_number: 0,
            compression_method,
            compressor: CompressionMethod::from_compression_method(compression_method)?,
            file_comment: None,
            extra_fields: Vec::new(),
            has_zip64_extra_field: false,
        };

        Ok(archive_file_entry)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buffer
    }
}

pub fn is_streaming(flags: u16) -> bool {
    flags & EXTENDED_LOCAL_HEADER_FLAG != 0
}

pub struct ArchiveDescriptorReader {
    index: usize,
}

macro_rules! read_type {
    ($self:expr, $stream:expr, $typ:ty) => {{
        let upper_bound = $self.index + ::std::mem::size_of::<$typ>();

        let read: [u8; ::std::mem::size_of::<$typ>()] =
            $stream[$self.index..upper_bound].try_into().unwrap();
        let value = <$typ>::from_le_bytes(read);

        $self.index = upper_bound;

        let type_str = stringify!($typ);
        println!(
            "read_{} value: {:} new index {:}",
            type_str, value, $self.index
        );

        value
    }};
}
impl ArchiveDescriptorReader {
    pub fn new() -> ArchiveDescriptorReader {
        ArchiveDescriptorReader { index: 0 }
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn read_u32(&mut self, stream: &[u8]) -> u32 {
        read_type!(self, stream, u32)
    }

    pub fn read_i32(&mut self, stream: &[u8]) -> i32 {
        read_type!(self, stream, i32)
    }

    pub fn read_u16(&mut self, stream: &[u8]) -> u16 {
        read_type!(self, stream, u16)
    }

    pub fn read_u8(&mut self, stream: &[u8]) -> u8 {
        read_type!(self, stream, u8)
    }

    pub fn read_u64(&mut self, stream: &[u8]) -> u64 {
        read_type!(self, stream, u64)
    }

    pub fn read_utf8_string(&mut self, stream: &[u8], string_len: usize) -> String {
        let upper_bound = self.index + string_len;

        println!(
            "read_utf8_string lb: {:?} up: {:} ({:} bytes) from a {:} length array.",
            self.index,
            upper_bound,
            string_len,
            stream.len()
        );

        let value = match str::from_utf8(&stream[self.index..upper_bound]) {
            Ok(v) => v.to_owned(),
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        self.index = upper_bound;

        println!(
            "read_utf8_string value: {:?} new index {:}",
            value, self.index
        );

        value
    }

    pub fn read_bytes(&mut self, stream: &[u8], len: usize) -> Vec<u8> {
        let upper_bound = self.index + len;

        println!(
            "read_bytes lb: {:?} up: {:} ({:} bytes) from a {:} length array.",
            self.index,
            upper_bound,
            len,
            stream.len()
        );

        let value = stream[self.index..upper_bound].to_owned();

        self.index = upper_bound;

        println!("read_bytes value: {:?} new index {:}", value, self.index);

        value
    }
}
#[derive(Debug, Default)]
pub struct CentralDirectoryEnd {
    pub number_of_this_disk: u32,
    pub number_of_the_disk_with_central_directory: u32,
    pub total_number_of_entries_on_this_disk: u64,
    pub total_number_of_entries_in_the_central_directory: u64,
    pub central_directory_size: u64,
    pub offset_of_start_of_central_directory: u64,
    pub z64ecdl_number_of_the_disk_with_the_start_of_the_zip64_end_of_central_directory: u32,
    pub z64ecdl_relative_offset_of_the_zip64_end_of_central_directory_record: u64,
    pub z64ecdl_total_number_of_disks: u32,
    pub archive_comment: Option<Vec<u8>>,
}

impl CentralDirectoryEnd {
    pub fn zip_file_comment_length(&self) -> u16 {
        match &self.archive_comment {
            Some(comment) => comment.len() as u16,
            None => 0,
        }
    }

    /// Set ZIP archive comment.
    ///
    /// This sets the raw bytes of the comment. The comment
    /// is typically expected to be encoded in UTF-8. Comment is truncated to 0xFFFF bytes.
    pub fn set_archive_comment(&mut self, comment: &str) {
        let bytes = comment.as_bytes();
        let len = std::cmp::min(bytes.len(), u16::MAX as usize);
        self.archive_comment = Some(bytes[0..len].to_owned());
    }

    // Per spec 4.4.1.4 - a CentralDirectoryEnd field might be insufficient to hold the
    // required data. In this case the file SHOULD contain a ZIP64 format record
    // and the field of this record will be set to -1
    pub fn needs_zip64_format_extensions(&self) -> bool {
        self.number_of_this_disk == 0xFFFF
            || self.number_of_the_disk_with_central_directory == 0xFFFF
            || self.total_number_of_entries_on_this_disk >= u16::MAX as u64
            || self.total_number_of_entries_in_the_central_directory >= u16::MAX as u64
            || self.central_directory_size >= u32::MAX as u64
            || self.offset_of_start_of_central_directory >= u32::MAX as u64
    }

    pub fn create_zip64_end_of_central_directory_record(
        &self,
        end_of_central_directory: &mut ArchiveDescriptor,
    ) {
        const SIZE_OF_THE_EOCD64_MINUS_12: u64 = 44;
        const VERSION_MADE_BY: u16 = 46;
        const MINIMUM_VERSION_NEEDED_TO_EXTRACT: u16 = 46;

        end_of_central_directory.write_u32(ZIP64_CENTRAL_DIRECTORY_END_SIGNATURE);
        end_of_central_directory.write_u64(SIZE_OF_THE_EOCD64_MINUS_12);
        end_of_central_directory.write_u16(VERSION_MADE_BY);
        end_of_central_directory.write_u16(MINIMUM_VERSION_NEEDED_TO_EXTRACT);
        end_of_central_directory.write_u32(self.number_of_this_disk);
        end_of_central_directory.write_u32(self.number_of_the_disk_with_central_directory);
        end_of_central_directory.write_u64(self.total_number_of_entries_on_this_disk);
        end_of_central_directory.write_u64(self.total_number_of_entries_in_the_central_directory);
        end_of_central_directory.write_u64(self.central_directory_size);
        end_of_central_directory.write_u64(self.offset_of_start_of_central_directory);
    }

    pub fn create_end_of_central_directory_locator(
        &mut self,
        end_of_central_directory: &mut ArchiveDescriptor,
    ) {
        self.z64ecdl_relative_offset_of_the_zip64_end_of_central_directory_record =
            self.offset_of_start_of_central_directory + self.central_directory_size;

        end_of_central_directory.write_u32(ZIP64_END_OF_CENTRAL_DIR_LOCATOR_SIGNATURE);
        end_of_central_directory.write_u32(
            self.z64ecdl_number_of_the_disk_with_the_start_of_the_zip64_end_of_central_directory,
        );
        end_of_central_directory
            .write_u64(self.z64ecdl_relative_offset_of_the_zip64_end_of_central_directory_record);
        end_of_central_directory.write_u32(self.z64ecdl_total_number_of_disks);
    }

    fn create_end_of_central_directory(&self, end_of_central_directory: &mut ArchiveDescriptor) {
        end_of_central_directory.write_u32(CENTRAL_DIRECTORY_END_SIGNATURE);
        end_of_central_directory.write_u16(self.number_of_this_disk.min(u16::MAX as u32) as u16);
        end_of_central_directory.write_u16(
            self.number_of_the_disk_with_central_directory
                .min(u16::MAX as u32) as u16,
        );
        end_of_central_directory.write_u16(
            self.total_number_of_entries_on_this_disk
                .min(u16::MAX as u64) as u16,
        );
        end_of_central_directory.write_u16(
            self.total_number_of_entries_in_the_central_directory
                .min(u16::MAX as u64) as u16,
        );

        end_of_central_directory.write_u32(self.central_directory_size.min(u32::MAX as u64) as u32);
        end_of_central_directory.write_u32(
            self.offset_of_start_of_central_directory
                .min(u32::MAX as u64) as u32,
        );

        if let Some(comment) = &self.archive_comment {
            end_of_central_directory.write_u16(comment.len() as u16);
            end_of_central_directory.write_bytes(comment);
        } else {
            end_of_central_directory.write_u16(0);
        };

        println!("EOCD\n {:?}", self)
    }
}

pub trait ExtraFields: Debug + Send + Sync {
    fn file_header_extra_field_size(&self) -> u16;
    fn central_header_extra_field_size(&self) -> u16;
    fn file_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    );
    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    );
}

//The central-directory extra field contains:
//- A subfield with ID 0x5455 (universal time) and 5 data bytes.
//  The local extra field has UTC/GMT modification/access times.
//- A subfield with ID 0x7875 (Unix UID/GID (any size)) and 11 data bytes:
//  01 04 e8 03 00 00 04 e8 03 00 00.

///
/// The time values are in standard Unix signed-long format, indicating
/// the number of seconds since 1 January 1970 00:00:00.  The times
/// are relative to Coordinated Universal Time (UTC), also sometimes
/// referred to as Greenwich Mean Time (GMT).  To convert to local time,
/// the software must know the local timezone offset from UTC/GMT.
///
/// Use the field definition given in Info-Zip's source archive: zip-3.0.tar.gz/proginfo/extrafld.txt.
/// It can be found here (https://github.com/LuaDist/zip/blob/master/proginfo/extrafld.txt)
///
#[derive(Debug, Default)]
pub struct ExtraFieldExtendedTimestamp {
    create_time: Option<i32>,
    access_time: Option<i32>,
    modify_time: Option<i32>,
    flags: u8,
}

impl ExtraFieldExtendedTimestamp {
    pub const HEADER_ID: u16 = X5455_EXTENDEDTIMESTAMP;

    /// The bit set inside the flags by when the last modification time is present in this extra field.
    const MODIFY_TIME_BIT: u8 = 1;

    ///  The bit set inside the flags by when the lasr access time is present in this extra field.
    const ACCESS_TIME_BIT: u8 = 2;

    /// The bit set inside the flags by when the original creation time is present in this extra field.
    const CREATE_TIME_BIT: u8 = 4;

    fn new(modify_time: Option<i32>, access_time: Option<i32>, create_time: Option<i32>) -> Self {
        let mut default = Self::default();

        default.set_modify_time(modify_time);
        default.set_access_time(access_time);
        default.set_create_time(create_time);

        default
    }

    fn set_modify_time(&mut self, modify_time: Option<i32>) {
        self.modify_time = modify_time;

        if modify_time.is_some() {
            self.flags |= ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT;
        } else {
            self.flags &= !ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT;
        }
    }

    fn set_access_time(&mut self, access_time: Option<i32>) {
        self.access_time = access_time;

        if access_time.is_some() {
            self.flags |= ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT;
        } else {
            self.flags &= !ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT;
        }
    }

    fn set_create_time(&mut self, create_time: Option<i32>) {
        self.create_time = create_time;

        if create_time.is_some() {
            self.flags |= ExtraFieldExtendedTimestamp::CREATE_TIME_BIT;
        } else {
            self.flags &= !ExtraFieldExtendedTimestamp::CREATE_TIME_BIT;
        }
    }

    fn file_header_extra_field_data_size(&self) -> u16 {
        let mut size: u16 = 1; //for flags
        size += (self.flags.count_ones() * 4) as u16;
        size
    }

    fn central_header_extra_field_data_size(&self) -> u16 {
        let mut size: u16 = 1; //for flags
        size +=
            ((self.flags & ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT).count_ones() * 4) as u16;
        size
    }

    pub fn parse_extra_field(
        indexer: &mut ArchiveDescriptorReader,
        extra_field_as_bytes: &[u8],
        extra_field_data_size: u16,
    ) -> Self {
        let mut create_time: Option<i32> = None;
        let mut access_time: Option<i32> = None;
        let mut modify_time: Option<i32> = None;
        let mut flags: u8 = 0;

        match extra_field_data_size {
            0 => {}
            1..=4 => flags = indexer.read_u8(extra_field_as_bytes),
            5..=8 => {
                flags = indexer.read_u8(extra_field_as_bytes);
                modify_time = Some(indexer.read_i32(extra_field_as_bytes))
            }
            9..=13 => {
                flags = indexer.read_u8(extra_field_as_bytes);
                modify_time = Some(indexer.read_i32(extra_field_as_bytes));
                access_time = Some(indexer.read_i32(extra_field_as_bytes))
            }
            _ => {
                flags = indexer.read_u8(extra_field_as_bytes);
                modify_time = Some(indexer.read_i32(extra_field_as_bytes));
                access_time = Some(indexer.read_i32(extra_field_as_bytes));
                create_time = Some(indexer.read_i32(extra_field_as_bytes))
            }
        }

        Self {
            create_time,
            access_time,
            modify_time,
            flags,
        }
    }
}

impl ExtraFields for ExtraFieldExtendedTimestamp {
    fn file_header_extra_field_size(&self) -> u16 {
        4 + self.file_header_extra_field_data_size()
    }

    fn central_header_extra_field_size(&self) -> u16 {
        4 + self.central_header_extra_field_data_size()
    }

    fn file_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        self.central_header_extra_write_data(archive_descriptor, archive_file_entry);

        if let Some(access_time) = self.access_time {
            archive_descriptor.write_i32(access_time);
        }

        if let Some(create_time) = self.create_time {
            archive_descriptor.write_i32(create_time);
        }
    }

    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        _archive_file_entry: &ArchiveFileEntry,
    ) {
        archive_descriptor.write_u16(ExtraFieldExtendedTimestamp::HEADER_ID);
        archive_descriptor.write_u16(self.file_header_extra_field_data_size());
        archive_descriptor.write_u8(self.flags); //     The bit set inside the flags by when the last modification time is present in this extra field.

        if let Some(modify_time) = self.modify_time {
            archive_descriptor.write_i32(modify_time);
        }
    }
}

/// The following is the layout of the ZIP64 extended
/// information "extra" block. If one of the size or
/// offset fields in the Local or Central directory
/// record is too small to hold the required data,
/// a ZIP64 extended information record is created.
/// The order of the fields in the ZIP64 extended
/// information record is fixed, but the fields will
/// only appear if the corresponding Local or Central
/// directory record field is set to 0xFFFF or 0xFFFFFFFF.
///
/// If one entry does not fit into the classic LOC or CEN record, only that entry is required to be moved into a ZIP64 extra field. The other entries may stay in the classic record. Therefore, not all entries shown in the following table might be stored in a ZIP64 extra field. However, if they appear, their order must be as shown in the table.
///
/// Note: all fields stored in Intel low-byte/high-byte order.
#[derive(Debug)]
pub struct ExtraFieldZIP64ExtendedInformation {}

impl ExtraFieldZIP64ExtendedInformation {
    pub const HEADER_ID: u16 = 0x0001;
    const ZIP64_EXTRA_FIELD_SIZE: u16 = 8 * 3 + 4;
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_extra_field(
        indexer: &mut ArchiveDescriptorReader,
        extra_field_as_bytes: &[u8],
        extra_field_data_size: u16,
        archive_file_entry: &mut ArchiveFileEntry,
    ) -> Self {
        match extra_field_data_size {
            0..=7 => { //Nothing worthy}
            }
            8..=15 => archive_file_entry.uncompressed_size = indexer.read_u64(extra_field_as_bytes),
            16..=23 => {
                archive_file_entry.uncompressed_size = indexer.read_u64(extra_field_as_bytes);
                archive_file_entry.compressed_size = indexer.read_u64(extra_field_as_bytes);
            }
            24..=31 => {
                archive_file_entry.uncompressed_size = indexer.read_u64(extra_field_as_bytes);
                archive_file_entry.compressed_size = indexer.read_u64(extra_field_as_bytes);
                archive_file_entry.offset = indexer.read_u64(extra_field_as_bytes);
            }
            _ => {
                archive_file_entry.uncompressed_size = indexer.read_u64(extra_field_as_bytes);
                archive_file_entry.compressed_size = indexer.read_u64(extra_field_as_bytes);
                archive_file_entry.offset = indexer.read_u64(extra_field_as_bytes);
                archive_file_entry.file_disk_number = indexer.read_u32(extra_field_as_bytes);
            }
        }

        Self::new()
    }
}

impl ExtraFields for ExtraFieldZIP64ExtendedInformation {
    fn file_header_extra_field_size(&self) -> u16 {
        todo!()
    }

    fn central_header_extra_field_size(&self) -> u16 {
        todo!()
    }

    fn file_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        if archive_file_entry.uncompressed_size == 0 {
            archive_descriptor
                .write_zeros(ExtraFieldZIP64ExtendedInformation::ZIP64_EXTRA_FIELD_SIZE as usize);
        } else {
            self.central_header_extra_write_data(archive_descriptor, archive_file_entry);
        }
    }

    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        let size = if archive_file_entry.file_disk_number >= u16::MAX as u32 {
            28
        } else if archive_file_entry.offset >= u32::MAX as u64 {
            24
        } else if archive_file_entry.compressed_size >= u32::MAX as u64 {
            16
        } else {
            8
        };

        archive_descriptor.write_u16(ExtraFieldZIP64ExtendedInformation::HEADER_ID);
        archive_descriptor
            .write_u16(ExtraFieldZIP64ExtendedInformation::ZIP64_EXTRA_FIELD_SIZE.min(size));

        archive_descriptor.write_u64(archive_file_entry.uncompressed_size);

        if size >= 16 {
            archive_descriptor.write_u64(archive_file_entry.compressed_size);
            if size >= 24 {
                archive_descriptor.write_u64(archive_file_entry.offset);
                if size >= 28 {
                    archive_descriptor.write_u32(archive_file_entry.file_disk_number);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ExtraFieldUnknown {
    header_id: u16,
    data: Vec<u8>,
}

impl ExtraFieldUnknown {
    pub fn parse_extra_field(
        indexer: &mut ArchiveDescriptorReader,
        extra_field_as_bytes: &[u8],
        extra_field_data_size: u16,
        header_id: u16,
    ) -> Self {
        let data = indexer.read_bytes(extra_field_as_bytes, extra_field_data_size as usize);
        Self { header_id, data }
    }
}

impl ExtraFields for ExtraFieldUnknown {
    fn file_header_extra_field_size(&self) -> u16 {
        self.central_header_extra_field_size()
    }

    fn central_header_extra_field_size(&self) -> u16 {
        self.data.len() as u16
    }

    fn file_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        self.central_header_extra_write_data(archive_descriptor, archive_file_entry)
    }

    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        _archive_file_entry: &ArchiveFileEntry,
    ) {
        archive_descriptor.write_u16(self.header_id);
        archive_descriptor.write_u16(self.central_header_extra_field_size());
        archive_descriptor.write_bytes(&self.data);
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::constants::LOCAL_FILE_HEADER_SIGNATURE;

    #[test]
    fn test_write_file_header() {
        let version_needed = CompressionMethod::Deflate().zip_version_needed();
        let compression_method = CompressionMethod::Deflate().zip_code();
        let general_purpose_flags: u16 = 1 << 3 | 1 << 11;
        let time = 0u16;
        let date = 0u16;
        let crc = 0u32;
        let compressed_size = 0u32;
        let uncompressed_size = 0u32;
        let file_name = "file1.txt";
        let file_name_len = file_name.as_bytes().len() as u16;

        println!("file_name {:?} length: {:}", file_name, file_name_len);
        let extra_field_length = 0u16;

        let mut desc = ArchiveDescriptor::new(100);
        desc.write_u32(LOCAL_FILE_HEADER_SIGNATURE);
        desc.write_u16(version_needed);
        desc.write_u16(general_purpose_flags);
        desc.write_u16(compression_method);
        desc.write_u16(time);
        desc.write_u16(date);
        desc.write_u32(crc);
        desc.write_u32(compressed_size);
        desc.write_u32(uncompressed_size);
        desc.write_u16(file_name_len);
        desc.write_u16(extra_field_length);
        desc.write_str(file_name);
        let vec = desc.finish();

        println!("desc len {:} \n {:02X?}", &vec.len(), &vec);

        let entry = ArchiveDescriptor::read_file_descriptor(&vec);

        print!("{:#?}", entry)
    }

    #[test]
    fn test_mem_dump() {
        let vec: Vec<u8> = vec![
            0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00,
            0x1D, 0x85, 0xB7, 0xB3, 0xB9, 0x36, 0x00, 0x00, 0xDF, 0xE0, 0x3E, 0x00, 0x09, 0x00,
            0x00, 0x00, 0x66, 0x69, 0x6C, 0x65, 0x31, 0x2E, 0x74, 0x78, 0x74, 0xED, 0xCD, 0x39,
            0x11, 0x00, 0x20, 0x0C, 0x00, 0xB0, 0x1D, 0x35, 0x14, 0xCA, 0xE7, 0xDF, 0x18, 0x2A,
            0x18, 0xB8, 0xCB, 0x96, 0x2D, 0xD1, 0x7A, 0x8E, 0xB9, 0xF6, 0xA9, 0xF1, 0x4C, 0x25,
            0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48, 0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48,
            0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48, 0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48,
            0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48, 0x24, 0x12, 0x89, 0x44, 0x22,
        ];

        let entry = ArchiveDescriptor::read_file_descriptor(&vec);

        print!("{:#?}", entry)
    }

    #[test]
    fn test_mem_dump2() {
        let vec: Vec<u8> = vec![
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x1d, 0x85, 0xb7, 0xb3, 0xc6, 0x36, 0x00, 0x00, 0xdf, 0xe0, 0x3e, 0x00, 0x09, 0x00,
            0x00, 0x00, 0x66, 0x69, 0x6c, 0x65, 0x31, 0x2e, 0x74, 0x78, 0x74, 0x78, 0x9c, 0xec,
            0xcd, 0x39, 0x11, 0x00, 0x20, 0x0c,
        ];
        let entry = ArchiveDescriptor::read_file_descriptor(&vec);

        print!("{:#?}", entry)
    }

    #[test]
    fn test_mem_dump3() {
        let vec: Vec<u8> = vec![
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x1d, 0x85, 0xb7, 0xb3, 0xc6, 0x36, 0x00, 0x00, 0xdf, 0xe0, 0x3e, 0x00, 0x09, 0x00,
            0x00, 0x00, 0x66, 0x69, 0x6c, 0x65, 0x31, 0x2e, 0x74, 0x78, 0x74, 0x78, 0x9c, 0xec,
            0xcd, 0x39, 0x11, 0x00, 0x20, 0x0c,
        ];
        let entry = ArchiveDescriptor::read_file_descriptor(&vec).unwrap();

        println!("{:#?}", entry);

        println!("\nFile descriptor\n{}", entry);
    }

    #[test]
    fn test_mem_dump_cur_lib_sf() {
        let vec: Vec<u8> = vec![
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x82, 0xea, 0xc6, 0x30, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x13, 0x00,
            0x00, 0x00, 0x73, 0x68, 0x6f, 0x72, 0x74, 0x5f, 0x74, 0x65, 0x78, 0x74, 0x5f, 0x66,
            0x69, 0x6c, 0x65, 0x2e, 0x74, 0x78, 0x74, 0x78, 0x9c, 0xec, 0xcd, 0xb9, 0x11, 0x00,
            0x30, 0x08, 0x03, 0xb0, 0x3e, 0xd3, 0xc4, 0xfc, 0xec, 0xbf, 0x18, 0x53, 0x70, 0x47,
            0xe1, 0x4e, 0x9d, 0x20, 0x6a, 0x1e, 0x59, 0xfd, 0xb1, 0xa6, 0x07, 0x26, 0x4c, 0x98,
            0x5c, 0x4c, 0x06, 0x00, 0x00, 0xff, 0xff, 0x03, 0x00, 0x6b, 0x19, 0xd0, 0x50, 0x50,
            0x4b, 0x01, 0x02, 0x2e, 0x03, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x82, 0xea, 0xc6, 0x30, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x13,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xa4, 0x81, 0x00,
            0x00, 0x00, 0x00, 0x73, 0x68, 0x6f, 0x72, 0x74, 0x5f, 0x74, 0x65, 0x78, 0x74, 0x5f,
            0x66, 0x69, 0x6c, 0x65, 0x2e, 0x74, 0x78, 0x74, 0x50, 0x4b, 0x05, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x41, 0x00, 0x00, 0x00, 0x61, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let entry = ArchiveDescriptor::read_file_descriptor(&vec).unwrap();

        println!("{:#?}", entry);

        println!("\nFile descriptor\n{}", entry);
    }

    #[test]
    fn test_mem_dump_rust_zip_lib_sf() {
        let vec: Vec<u8> = vec![
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00,
            0x00, 0x82, 0xea, 0xc6, 0x24, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x13, 0x00,
            0x00, 0x00, 0x73, 0x68, 0x6f, 0x72, 0x74, 0x5f, 0x74, 0x65, 0x78, 0x74, 0x5f, 0x66,
            0x69, 0x6c, 0x65, 0x2e, 0x74, 0x78, 0x74, 0xed, 0xcd, 0xb9, 0x11, 0x00, 0x30, 0x08,
            0x03, 0xb0, 0x3e, 0xd3, 0xc4, 0xfc, 0xec, 0xbf, 0x18, 0x53, 0x70, 0x47, 0xe1, 0x4e,
            0x9d, 0x20, 0x6a, 0x1e, 0x59, 0xfd, 0xb1, 0xa6, 0x07, 0x26, 0x4c, 0x98, 0x5c, 0x4c,
            0x06, 0x50, 0x4b, 0x01, 0x02, 0x2e, 0x03, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
            0x00, 0x21, 0x00, 0x00, 0x82, 0xea, 0xc6, 0x24, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xa4,
            0x81, 0x00, 0x00, 0x00, 0x00, 0x73, 0x68, 0x6f, 0x72, 0x74, 0x5f, 0x74, 0x65, 0x78,
            0x74, 0x5f, 0x66, 0x69, 0x6c, 0x65, 0x2e, 0x74, 0x78, 0x74, 0x50, 0x4b, 0x05, 0x06,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x41, 0x00, 0x00, 0x00, 0x55, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let entry = ArchiveDescriptor::read_file_descriptor(&vec).unwrap();

        println!("{:#?}", entry);

        println!("\nFile descriptor\n{}", entry);
    }

    #[test]
    fn test_mem_dump_rust_zip_lib_lzma() {
        let vec: Vec<u8> = vec![
            0x50, 0x4b, 0x03, 0x04, 0x3f, 0x03, 0x00, 0x00, 0x0e, 0x00, 0xf1, 0xb1, 0x66, 0x56,
            0x77, 0xe6, 0x34, 0x6b, 0x82, 0x05, 0x00, 0x00, 0xea, 0x0c, 0x00, 0x00, 0x06, 0x00,
            0x00, 0x00, 0x65, 0x78, 0x2e, 0x74, 0x78, 0x74, 0x10, 0x02, 0x05, 0x00, 0x5d, 0x00,
            0x10, 0x00, 0x00, 0x00, 0x26, 0x1b, 0xca, 0x46, 0x67, 0x5a, 0xf2, 0x77, 0xb8, 0x7d,
            0x86, 0xd8, 0x41, 0xdb, 0x05, 0x35, 0xcd, 0x83, 0xa5, 0x7c, 0x12, 0xa5, 0x05, 0xdb,
            0x90, 0xbd, 0x2f, 0x14, 0xd3, 0x71, 0x72, 0x96, 0xa8, 0x8a, 0x7d, 0x84, 0x56, 0x71,
            0x8d, 0x6a, 0x22, 0x98, 0xab, 0x9e, 0x3d, 0xc3, 0x55, 0xef, 0xcc, 0xa5, 0xc3, 0xdd,
            0x76, 0xd0, 0x6b, 0x74, 0x6a, 0x91, 0x2b, 0xcb, 0x12, 0x8d, 0x9e, 0x09, 0x28, 0xe7,
            0x95, 0x6b, 0x23, 0x10, 0x99, 0xad, 0x6f, 0x77, 0x01, 0x3b, 0xbd, 0x8f, 0xb1, 0xcf,
            0x42, 0xa9, 0x6f, 0x17, 0xff, 0x29, 0x8c, 0x93, 0x48, 0xa2, 0x3d, 0x6c, 0x52, 0x66,
            0x68, 0x7a, 0x10, 0x56, 0xd7, 0x8d, 0xd4, 0xb5, 0xff, 0xa7, 0x6c, 0x16, 0xa5, 0x47,
            0xa8, 0x67, 0x5d, 0x40, 0xb5, 0x6f,
        ];
        let entry = ArchiveDescriptor::read_file_descriptor(&vec).unwrap();

        println!("{:#?}", entry);

        println!("\nFile descriptor\n{}", entry);
    }
}
