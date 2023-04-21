use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str;
use std::u32;
use std::u8;

use chrono::NaiveDateTime;
use chrono::{DateTime, Local, TimeZone, Utc};

use super::compression::CompressionMethod;

use crate::constants::CENTRAL_DIRECTORY_END_SIGNATURE;
use crate::constants::MS_DIR;
use crate::constants::S_IFDIR;
use crate::constants::VERSION_USES_ZIP64_FORMAT_EXTENSIONS;
use crate::constants::X5455_EXTENDEDTIMESTAMP;
use crate::constants::ZIP64_CENTRAL_DIRECTORY_END_SIGNATURE;

use crate::constants::ZIP64_END_OF_CENTRAL_DIR_LOCATOR_SIGNATURE;
#[cfg(any(feature = "experimental"))]
use crate::error::ArchiveError;
use crate::types::DateTimeCS;
use crate::types::FileCompatibilitySystem;

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

    pub fn write_bytes(&mut self, val: &[u8]) {
        self.buffer.extend_from_slice(val);
    }

    pub fn write_zeros(&mut self, len: usize) {
        self.buffer.resize(self.len() + len, 0);
    }

    #[cfg(any(feature = "experimental"))]
    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[cfg(any(feature = "experimental"))]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    #[cfg(any(feature = "experimental"))]
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

#[cfg(any(feature = "experimental"))]
#[derive(Default)]
pub struct ArchiveDescriptorReader {
    index: usize,
}

#[cfg(any(feature = "experimental"))]
macro_rules! read_type {
    ($self:expr, $stream:expr, $typ:ty) => {{
        let upper_bound = $self.index + ::std::mem::size_of::<$typ>();

        let read: [u8; ::std::mem::size_of::<$typ>()] =
            match $stream[$self.index..upper_bound].try_into() {
                Ok(v) => v,
                Err(e) => {
                    println!("slice with incorrect length {:?}", e);
                    Default::default()
                }
            };
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

#[cfg(any(feature = "experimental"))]
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
    #[cfg(any(feature = "experimental"))]
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
        self.number_of_this_disk >= u16::MAX as u32
            || self.number_of_the_disk_with_central_directory >= u16::MAX as u32
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

    pub fn create_end_of_central_directory(
        &self,
        end_of_central_directory: &mut ArchiveDescriptor,
    ) {
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

        println!("EOCD\n {:#?}", self)
    }
}

pub trait ExtraField: Debug + Send + Sync {
    fn local_header_extra_field_size(&self, archive_file_entry: &ArchiveFileEntry) -> u16;
    fn central_header_extra_field_size(&self, archive_file_entry: &ArchiveFileEntry) -> u16;
    fn local_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    );

    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    );

    fn as_any(&self) -> &dyn Any;

    fn display_central(&self) -> String;
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
    flags: u8,
    modify_time: Option<i32>,
    access_time: Option<i32>,
    create_time: Option<i32>,
}

impl ExtraFieldExtendedTimestamp {
    pub const HEADER_ID: u16 = X5455_EXTENDEDTIMESTAMP;

    /// The bit set inside the flags by when the last modification time is present in this extra field.
    const MODIFY_TIME_BIT: u8 = 1;

    ///  The bit set inside the flags by when the lasr access time is present in this extra field.
    const ACCESS_TIME_BIT: u8 = 2;

    /// The bit set inside the flags by when the original creation time is present in this extra field.
    const CREATE_TIME_BIT: u8 = 4;

    pub fn new(
        modify_time: Option<i32>,
        access_time: Option<i32>,
        create_time: Option<i32>,
    ) -> Self {
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

    pub fn modified_time_utc(&self) -> Option<String> {
        match self.modify_time {
            Some(time) => {
                if let Some(datetime) = NaiveDateTime::from_timestamp_opt(time as i64, 0) {
                    let dt = DateTime::<Utc>::from_utc(datetime, Utc);
                    Some(dt.to_string())
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn modified_time_local(&self) -> Option<String> {
        match self.modify_time {
            Some(time) => match Local.timestamp_opt(time as i64, 0) {
                chrono::LocalResult::None => None,
                chrono::LocalResult::Single(dt) => Some(dt.to_string()),
                chrono::LocalResult::Ambiguous(dt, _) => Some(dt.to_string()),
            },
            None => None,
        }
    }

    #[cfg(any(feature = "experimental"))]
    pub fn parse_extra_field(
        indexer: &mut ArchiveDescriptorReader,
        extra_field_as_bytes: &[u8],
        extra_field_data_size: u16,
    ) -> Self {
        let mut flags: u8 = 0;
        let mut modify_time: Option<i32> = None;
        let mut access_time: Option<i32> = None;
        let mut create_time: Option<i32> = None;

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

    fn central_header_extra_write_data_common(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,

        extra_field_data_size: u16,
    ) {
        if self.flags == 0 {
            return;
        }

        archive_descriptor.write_u16(ExtraFieldExtendedTimestamp::HEADER_ID);
        archive_descriptor.write_u16(extra_field_data_size);
        archive_descriptor.write_u8(self.flags); //     The bit set inside the flags by when the last modification time is present in this extra field.

        if let Some(modify_time) = self.modify_time {
            archive_descriptor.write_i32(modify_time);
        }
    }
}

impl ExtraField for ExtraFieldExtendedTimestamp {
    fn local_header_extra_field_size(&self, _archive_file_entry: &ArchiveFileEntry) -> u16 {
        4 + self.file_header_extra_field_data_size()
    }

    fn central_header_extra_field_size(&self, _archive_file_entry: &ArchiveFileEntry) -> u16 {
        4 + self.central_header_extra_field_data_size()
    }

    fn local_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        _archive_file_entry: &ArchiveFileEntry,
    ) {
        if self.flags == 0 {
            return;
        }

        self.central_header_extra_write_data_common(
            archive_descriptor,
            self.file_header_extra_field_data_size(),
        );

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
        if self.flags == 0 {
            return;
        }

        self.central_header_extra_write_data_common(
            archive_descriptor,
            self.central_header_extra_field_data_size(),
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn display_central(&self) -> String {
        let mut plural = "";

        let mut times = String::new();

        if self.flags & ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT != 0 {
            times.push_str("modification")
        }

        if self.flags & ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT != 0 {
            if !times.is_empty() {
                times.push('/');
                plural = "s";
            }
            times.push_str("access")
        }

        if self.flags & ExtraFieldExtendedTimestamp::CREATE_TIME_BIT != 0 {
            if !times.is_empty() {
                times.push('/');
                plural = "s";
            }
            times.push_str("creation")
        }

        format!(
            "- A subfield with ID 0x{:04X} (universal time) and {} data bytes.
  The local extra field has UTC/GMT {} time{}.",
            ExtraFieldExtendedTimestamp::HEADER_ID,
            self.central_header_extra_field_data_size(),
            times,
            plural
        )
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
/// If one entry does not fit into the classic LOC or CEN record,
/// _only that entry is required_ to be moved into a ZIP64 extra
/// field. The other entries may stay in the classic record.
/// Therefore, not all entries shown in the following table
/// might be stored in a ZIP64 extra field. However,
/// if they appear, their order must be as shown in the table.
///
/// Note: all fields stored in Intel low-byte/high-byte order.
///
/// This entry in the Local header must include BOTH original
/// and compressed file sizes.
#[derive(Debug, Default)]
pub struct ExtraFieldZIP64ExtendedInformation {
    parsed_sized: u16,
}

impl ExtraFieldZIP64ExtendedInformation {
    pub const HEADER_ID: u16 = 0x0001;
    const ZIP64_EXTRA_FIELD_SIZE: u16 = 8 * 3 + 4;

    #[cfg(any(feature = "experimental"))]
    pub fn new(parsed_sized: u16) -> Self {
        Self { parsed_sized }
    }

    #[cfg(any(feature = "experimental"))]
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

        Self::new(extra_field_data_size)
    }
}

impl ExtraField for ExtraFieldZIP64ExtendedInformation {
    fn local_header_extra_field_size(&self, _archive_file_entry: &ArchiveFileEntry) -> u16 {
        16
    }

    fn central_header_extra_field_size(&self, archive_file_entry: &ArchiveFileEntry) -> u16 {
        let size: u16 = if archive_file_entry.file_disk_number >= u16::MAX as u32 {
            28
        } else if archive_file_entry.offset >= u32::MAX as u64 {
            24
        } else if archive_file_entry.compressed_size >= u32::MAX as u64 {
            16
        } else if archive_file_entry.uncompressed_size >= u32::MAX as u64 {
            8
        } else {
            0
        };
        size
    }

    fn local_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        let size = self.local_header_extra_field_size(archive_file_entry);
        //If uncompressed size == 0 it do zero padding
        if archive_file_entry.uncompressed_size == 0 {
            archive_descriptor.write_zeros(size as usize);
        } else {
            self.central_header_extra_write_data(archive_descriptor, archive_file_entry);
            archive_descriptor.write_u16(ExtraFieldZIP64ExtendedInformation::HEADER_ID);
            archive_descriptor.write_u16(size);
            archive_descriptor.write_u64(archive_file_entry.uncompressed_size);
            archive_descriptor.write_u64(archive_file_entry.compressed_size);
        }
    }

    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        let size = self.central_header_extra_field_size(archive_file_entry);

        if size == 0 {
            return;
        }

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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn display_central(&self) -> String {
        format!(
            "- A subfield with ID 0x{:04X} (Zip64) and {} data bytes.",
            ExtraFieldZIP64ExtendedInformation::HEADER_ID,
            self.parsed_sized,
        )
    }
}

#[derive(Debug)]
pub struct ExtraFieldUnknown {
    header_id: u16,
    data: Vec<u8>,
}

impl ExtraFieldUnknown {
    #[cfg(any(feature = "experimental"))]
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

impl ExtraField for ExtraFieldUnknown {
    fn local_header_extra_field_size(&self, archive_file_entry: &ArchiveFileEntry) -> u16 {
        self.central_header_extra_field_size(archive_file_entry)
    }

    fn central_header_extra_field_size(&self, _archive_file_entry: &ArchiveFileEntry) -> u16 {
        self.data.len() as u16
    }

    fn local_header_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        self.central_header_extra_write_data(archive_descriptor, archive_file_entry)
    }

    fn central_header_extra_write_data(
        &self,
        archive_descriptor: &mut ArchiveDescriptor,
        archive_file_entry: &ArchiveFileEntry,
    ) {
        archive_descriptor.write_u16(self.header_id);
        archive_descriptor.write_u16(self.central_header_extra_field_size(archive_file_entry));
        archive_descriptor.write_bytes(&self.data);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn display_central(&self) -> String {
        format!(
            "- A subfield with ID 0x{:04X} (Zip64) and {} data bytes.",
            self.header_id,
            -1, //WRONG BUT PLACEHOLDER
        )
    }
}

/// The archive file complete information.
///
/// Most of this information is located in the archive central registry and it's partly duplicated in thier respective file header.
#[derive(Debug, Default)]
pub struct ArchiveFileEntry {
    pub version_made_by: u16,
    pub minimum_version_needed_to_extract: u16,
    pub general_purpose_flags: u16,
    pub compression_method: u16,
    pub last_mod_file_time: u16,
    pub last_mod_file_date: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub file_name_len: u16,
    pub extra_field_length: u16,
    pub file_name_as_bytes: Vec<u8>,
    pub offset: u64,
    pub compressor: CompressionMethod,
    pub file_disk_number: u32,
    pub internal_file_attributes: u16,
    pub external_file_attributes: u32,
    pub file_comment: Option<Vec<u8>>,
    pub extra_fields: Vec<Box<dyn ExtraField>>,
}

impl ArchiveFileEntry {
    pub fn version_needed_to_extract(&self) -> u16 {
        if self.is_zip64() {
            self.minimum_version_needed_to_extract
                .max(VERSION_USES_ZIP64_FORMAT_EXTENSIONS)
        } else {
            self.minimum_version_needed_to_extract
        }
    }

    fn extended_local_header(&self) -> bool {
        self.general_purpose_flags & (1u16 << 3) != 0
    }

    fn is_encrypted(&self) -> bool {
        self.general_purpose_flags & (1u16 << 0) != 0
    }

    fn version_made_by_pretty(&self) -> (u8, u8) {
        ArchiveFileEntry::pretty_version(self.version_made_by)
    }

    fn minimum_version_needed_to_extract_pretty(&self) -> (u8, u8) {
        ArchiveFileEntry::pretty_version(self.minimum_version_needed_to_extract)
    }

    ///Retreive the version in a pretty format
    fn pretty_version(zip_version: u16) -> (u8, u8) {
        let version_part = zip_version.to_le_bytes()[0];
        let major = version_part / 10;
        let minor = version_part % 10;

        (major, minor)
    }

    pub(crate) fn file_comment_length(&self) -> u16 {
        match &self.file_comment {
            Some(comment) => comment.len() as u16,
            None => 0,
        }
    }

    fn system_origin(&self) -> String {
        let system_code = self.version_made_by.to_be_bytes()[0];
        FileCompatibilitySystem::from_u8(system_code).to_string()
    }

    #[cfg(any(feature = "experimental"))]
    pub fn get_file_name(&self) -> String {
        String::from_utf8_lossy(&self.file_name_as_bytes).to_string()
    }

    pub fn is_zip64(&self) -> bool {
        self.uncompressed_size >= u32::MAX as u64
            || self.offset >= u32::MAX as u64
            || self.compressed_size >= u32::MAX as u64
    }

    pub fn zip64_compressed_size(&self) -> u32 {
        self.compressed_size.min(u32::MAX as u64) as u32
    }

    pub fn zip64_uncompressed_size(&self) -> u32 {
        self.uncompressed_size.min(u32::MAX as u64) as u32
    }

    pub fn zip64_offset(&self) -> u32 {
        self.offset.min(u32::MAX as u64) as u32
    }

    const TEXT_INDICATOR: u16 = 1;

    pub fn apparently_text_file(&mut self, is_text: bool) {
        if is_text {
            self.internal_file_attributes |= ArchiveFileEntry::TEXT_INDICATOR
        } else {
            self.internal_file_attributes &= !ArchiveFileEntry::TEXT_INDICATOR
        }
    }

    #[cfg(any(feature = "experimental"))]
    pub fn is_dir(&mut self) {
        self.internal_file_attributes &= !ArchiveFileEntry::TEXT_INDICATOR
    }

    pub fn is_apparently_text_file(&self) -> bool {
        self.internal_file_attributes & ArchiveFileEntry::TEXT_INDICATOR != 0
    }

    pub fn get_extra_field_time_stamp(&self) -> Option<&ExtraFieldExtendedTimestamp> {
        for extra_field_box in self.extra_fields.iter() {
            if let Some(extra_field) = extra_field_box
                .as_any()
                .downcast_ref::<ExtraFieldExtendedTimestamp>()
            {
                return Some(extra_field);
            };
        }
        None
    }

    pub fn has_zip64_extra_field(&self) -> bool {
        for extra_field_box in self.extra_fields.iter() {
            if extra_field_box
                .as_any()
                .downcast_ref::<ExtraFieldZIP64ExtendedInformation>()
                .is_some()
            {
                return true;
            };
        }
        false
    }

    pub fn need_to_add_zip64_extra_field(&mut self) {
        if !self.has_zip64_extra_field() && self.is_zip64() {
            let zip_extra_field = ExtraFieldZIP64ExtendedInformation::default();
            self.extra_fields.push(Box::new(zip_extra_field));
        }
    }
}

impl Display for ArchiveFileEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let padding = 48;

        let file_name = String::from_utf8_lossy(&self.file_name_as_bytes);

        writeln!(f, "{}\n", file_name)?;

        writeln!(
            f,
            "{: <padding$}{}",
            "offset of local header from start of archive:", self.offset
        )?;

        writeln!(f, "{: <padding$}({:016X}h) bytes", "", self.offset)?;

        writeln!(
            f,
            "{: <padding$}{}",
            "file system or operating system of origin:",
            self.system_origin()
        )?;

        let (major, minor) = self.version_made_by_pretty();
        writeln!(
            f,
            "{: <padding$}{}.{}",
            "version of encoding software:", major, minor
        )?;

        let (major, minor) = self.minimum_version_needed_to_extract_pretty();
        writeln!(
            f,
            "{: <padding$}{}.{}",
            "minimum software version required to extract:", major, minor
        )?;

        writeln!(
            f,
            "{: <padding$}{:#016b}",
            "general purpose bit flag:", self.general_purpose_flags
        )?;

        let label = match CompressionMethod::from_compression_method(self.compression_method) {
            Ok(compressor) => compressor.label().to_owned(),
            Err(_) => {
                let str_val = self.compression_method.to_string();
                let mut val = String::from("unknown (");
                val.push_str(&str_val);
                val.push(')');
                val
            }
        };

        writeln!(f, "{: <padding$}{}", "compression method:", label)?;

        let extended_local_header = if self.is_encrypted() {
            "encrypted"
        } else {
            "not encrypted"
        };

        writeln!(
            f,
            "{: <padding$}{}",
            "file security status:", extended_local_header
        )?;

        let extended_local_header = if self.extended_local_header() {
            "yes"
        } else {
            "no"
        };

        writeln!(
            f,
            "{: <padding$}{}",
            "extended local header:", extended_local_header
        )?;

        let date_time = DateTimeCS::from_msdos(self.last_mod_file_date, self.last_mod_file_time);
        writeln!(
            f,
            "{: <padding$}{}",
            "file last modified on (DOS date/time):", date_time
        )?;

        if let Some(extra_field_timestamp) = self.get_extra_field_time_stamp() {
            if let Some(time) = extra_field_timestamp.modified_time_local() {
                writeln!(
                    f,
                    "{: <padding$}{}",
                    "file last modified on (UT extra field modtime):", time
                )?;
            }

            if let Some(time) = extra_field_timestamp.modified_time_utc() {
                writeln!(
                    f,
                    "{: <padding$}{}",
                    "file last modified on (UT extra field modtime):", time
                )?;
            }
        }
        /*         file last modified on (UT extra field modtime): 2023 Apr 19 09:40:34 local
        file last modified on (UT extra field modtime): 2023 Apr 19 13:40:34 UTC */

        writeln!(
            f,
            "{: <padding$}{:08x}",
            "32-bit CRC value (hex):", self.crc32
        )?;

        writeln!(
            f,
            "{: <padding$}{} bytes",
            "compressed size:", self.compressed_size
        )?;
        writeln!(
            f,
            "{: <padding$}{:} bytes",
            "uncompressed size:", self.uncompressed_size
        )?;

        writeln!(
            f,
            "{: <padding$}{:} characters",
            "length of filename:", self.file_name_len
        )?;

        writeln!(
            f,
            "{: <padding$}{:} bytes",
            "length of extra field:", self.extra_field_length
        )?;

        writeln!(
            f,
            "{: <padding$}{:} characters",
            "length of file comment:",
            self.file_comment_length()
        )?;

        writeln!(
            f,
            "{: <padding$}disk {:}",
            "disk number on which file begins:",
            (self.file_disk_number + 1)
        )?;

        let file_type = if self.is_apparently_text_file() {
            "text"
        } else {
            "binary"
        };

        writeln!(f, "{: <padding$}{:}", "apparent file type:", file_type)?;

        let unix_file_attributes = (self.external_file_attributes >> 16) & 0xFFFF;
        let label = format!("Unix file attributes ({:06o} octal):", unix_file_attributes);
        writeln!(
            f,
            "{: <padding$}{:}",
            label,
            readable_file_unix_attributes(unix_file_attributes)
        )?;

        let dos_file_attributes = self.external_file_attributes & 0xFF;
        let label = format!("MS-DOS file attributes ({:0X} hex): ", dos_file_attributes);
        writeln!(
            f,
            "{: <padding$}{:}",
            label,
            readable_file_dos_attributes(dos_file_attributes)
        )?;

        if !self.extra_fields.is_empty() {
            writeln!(
                f,
                "{: <padding$}",
                "\nThe central-directory extra field contains:",
            )?;

            for extra_fields_box in self.extra_fields.iter() {
                let extra_fields = extra_fields_box.as_ref();
                writeln!(f, "{}", extra_fields.display_central(),)?;
            }
        }

        if let Some(comment) = &self.file_comment {
            writeln!(
                f,
                "\n------------------------- file comment begins ----------------------------"
            )?;
            let s = String::from_utf8_lossy(comment);
            writeln!(f, "{}", s)?;

            writeln!(
                f,
                "-------------------------- file comment ends -----------------------------"
            )?;
        }

        Ok(())
    }
}

fn readable_file_unix_attributes(file_attributes: u32) -> String {
    let mut s: String = String::from("----------");

    if file_attributes & 1 != 0 {
        s.replace_range(9..10, "x");
    }

    if file_attributes & 2 != 0 {
        s.replace_range(8..9, "w");
    }

    if file_attributes & 4 != 0 {
        s.replace_range(7..8, "r");
    }

    if file_attributes & (1 << 3) != 0 {
        s.replace_range(6..7, "x");
    }

    if file_attributes & (2 << 3) != 0 {
        s.replace_range(5..6, "w");
    }

    if file_attributes & (4 << 3) != 0 {
        s.replace_range(4..5, "r");
    }

    if file_attributes & (1 << 6) != 0 {
        s.replace_range(3..4, "x");
    }

    if file_attributes & (2 << 6) != 0 {
        s.replace_range(2..3, "w");
    }

    if file_attributes & (4 << 6) != 0 {
        s.replace_range(1..2, "r");
    }

    if file_attributes & S_IFDIR != 0 {
        s.replace_range(0..1, "d");
    }
    s
}

fn readable_file_dos_attributes(file_attributes: u32) -> &'static str {
    if file_attributes == 0 {
        return "none";
    }

    if file_attributes & MS_DIR != 0 {
        return "dir";
    }

    ""
}
#[cfg(test)]
#[path = "./tests/external_fields.rs"]
mod external_fields_tests;

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
        desc.write_bytes(file_name.as_bytes());
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
    fn test_permision() {
        let val = 0o755;
        println!("{:o} {}", val, readable_file_unix_attributes(val));

        let val = 0o644;
        println!("{:o} {}", val, readable_file_unix_attributes(val));
    }
}
