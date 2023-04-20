use std::sync::Arc;

use crate::{
    archive_common::{
        ArchiveDescriptor, ArchiveFileEntry, CentralDirectoryEnd, ExtraField,
        ExtraFieldExtendedTimestamp, ExtraFieldZIP64ExtendedInformation,
    },
    compression::CompressionMethod,
    constants::{
        CENTRAL_DIRECTORY_ENTRY_SIGNATURE, DATA_DESCRIPTOR_SIGNATURE, DIR_DEFAULT,
        EXTENDED_LOCAL_HEADER_FLAG, FILE_DEFAULT, FILE_HEADER_BASE_SIZE,
        LOCAL_FILE_HEADER_SIGNATURE, MS_DIR, S_IFDIR, S_IFREG, VERSION_MADE_BY,
        ZIP64_DESCRIPTOR_SIZE,
    },
};

/// Fast routine for detection of plain text
///  (ASCII or an ASCII-compatible extension such as ISO-8859, UTF-8, etc.)
/// Author: Cosmin Truta.
///
/// See [txtvsbin.txt](https://github.com/LuaDist/zip/blob/master/proginfo/txtvsbin.txt) for more information.
pub fn is_text_buf(buffer: &[u8]) -> bool {
    let mut result = false;
    for c in buffer {
        if *c >= 32 {
            result = true;
        } else if (*c <= 6) || (*c >= 14 && *c <= 25) || (*c >= 28 && *c <= 31) {
            return false; // black-listed character found; stop
        }
    }
    result
}

macro_rules! compress_common {
    ( $encoder:expr, $hasher:expr, $reader:ident $($_await:tt)*) => {{
        let mut buf = vec![0; 4096];
        let mut total_read: u64 = 0;

        let mut read = $reader.read(&mut buf)$($_await)*?;
        let is_text = is_text_buf(&buf[..read]);

        while read != 0 {
            total_read += read as u64;
            $hasher.update(&buf[..read]);
            $encoder.write_all(&buf[..read])$($_await)*?;
            read = $reader.read(&mut buf)$($_await)*?;
        }
        (total_read, is_text)
    }};
}

macro_rules! compress_common_async {
    ( $encoder:expr, $hasher:expr, $reader:ident) => {{
        let (total_read, is_text) = compress_common!($encoder, $hasher, $reader.await);
        $encoder.flush().await?;
        $encoder.shutdown().await?;
        (total_read, is_text)
    }};
}

macro_rules! compress_common_std {
    ( $encoder:expr, $hasher:expr, $reader:ident) => {{
        let (total_read, is_text) = compress_common!($encoder, $hasher, $reader);
        $encoder.finish()?;
        (total_read, is_text)
    }};
}

macro_rules! write_async {
    ( $encoder:expr, $hasher:expr, $reader:ident) => {{
        let (total_read, is_text) = compress_common!($encoder, $hasher, $reader.await);
        $encoder.flush().await?;
        (total_read, is_text)
    }};
}

macro_rules! write_std {
    ( $encoder:expr, $hasher:expr, $reader:ident) => {{
        let (total_read, is_text) = compress_common!($encoder, $hasher, $reader);
        $encoder.flush()?;
        (total_read, is_text)
    }};
}

pub(crate) use compress_common;
pub(crate) use compress_common_async;
pub(crate) use compress_common_std;
pub(crate) use write_async;
pub(crate) use write_std;

use super::FileOptions;

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
    is_dir: bool,
) -> (ArchiveDescriptor, ArchiveFileEntry) {
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

    let mut minimum_version_needed_to_extract = compressor.zip_version_needed();
    let version_made_by = options.system.update_version_needed(VERSION_MADE_BY);

    let mut extra_fields: Vec<Arc<dyn ExtraField>> = Vec::new();

    let mut extrafield_zip64: Option<Arc<ExtraFieldZIP64ExtendedInformation>> = None;
    if options.large_file && !is_streaming(data.base_flags) {
        let ts = ExtraFieldZIP64ExtendedInformation::default();
        let b: Arc<ExtraFieldZIP64ExtendedInformation> = Arc::new(ts);
        extrafield_zip64 = Some(b.clone());
        extra_fields.push(b);
    }

    if options.last_modified_time.extended_timestamp()
        || options.last_creation_time.is_some()
        || options.last_access_time.is_some()
    {
        let ts = ExtraFieldExtendedTimestamp::new(
            options.last_modified_time.timestamp(),
            options.last_access_time,
            options.last_creation_time,
        );
        extra_fields.push(Arc::new(ts));
    }

    let (unix_ftype, default_permission, ms_dos_attr) = if is_dir {
        general_purpose_flags &= !EXTENDED_LOCAL_HEADER_FLAG;
        minimum_version_needed_to_extract = 20;
        (S_IFDIR, DIR_DEFAULT, MS_DIR)
    } else {
        (S_IFREG, FILE_DEFAULT, 0)
    };

    let unix_permissions = if let Some(permissions) = options.unix_permissions {
        permissions | unix_ftype
    } else {
        unix_ftype | default_permission
    };

    let external_file_attributes: u32 = (unix_permissions << 16) + ms_dos_attr;

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
        external_file_attributes,
        file_disk_number: 0,
        extra_fields,
        file_comment,
    };

    let mut extended_data_buffer = ArchiveDescriptor::new(500);

    if let Some(ref extra_field) = extrafield_zip64 {
        extra_field.local_header_write_data(&mut extended_data_buffer, &archive_file_entry);
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

    (file_header, archive_file_entry)
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
    central_directory_header.write_u32(file_info.external_file_attributes); // External file attributes (regular file / rw-r--r--).
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

pub fn is_streaming(flags: u16) -> bool {
    flags & EXTENDED_LOCAL_HEADER_FLAG != 0
}

#[cfg(test)]
mod test {
    use super::is_text_buf;

    #[test]
    fn all_text() {
        let res = is_text_buf(b"Some string data");

        assert!(res)
    }

    #[test]
    fn not_all_text() {
        let mut v = b"Some string data".to_vec();
        v.push(3u8);

        let res = is_text_buf(&v);

        assert!(!res)
    }
}
