use crate::archive_common::{
    ArchiveDescriptorReader, CentralDirectoryEnd, ExtraField, ExtraFieldExtendedTimestamp,
    ExtraFieldUnknown, ExtraFieldZIP64ExtendedInformation,
};
use crate::compression::CompressionMethod;
use crate::constants::{CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, CENTRAL_DIRECTORY_ENTRY_SIGNATURE};
use crate::types::ArchiveFileEntry;
use crate::{
    constants::{CENTRAL_DIRECTORY_END_SIGNATURE, END_OF_CENTRAL_DIRECTORY_SIZE},
    error::ArchiveError,
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt::{Debug, Display};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

pub struct ArchiveReader<R>
where
    R: Read + Seek,
{
    #[allow(dead_code)]
    reader: R,
    pub file_entries: Vec<ArchiveFileEntry>,
    pub central_directory_end: CentralDirectoryEnd,
}

impl<R: Read + Seek> ArchiveReader<R> {
    pub fn new(mut reader: R) -> Result<ArchiveReader<R>, ArchiveError> {
        let (central_directory_end, file_entries) = Self::parse(&mut reader)?;

        let ar = ArchiveReader {
            reader,
            file_entries,
            central_directory_end,
        };
        Ok(ar)
    }

    fn parse(reader: &mut R) -> Result<(CentralDirectoryEnd, Vec<ArchiveFileEntry>), ArchiveError> {
        //find central dir end

        let file_length = reader.seek(SeekFrom::End(0))?;

        let mut position: u64 = match file_length.checked_sub(END_OF_CENTRAL_DIRECTORY_SIZE) {
            Some(p) => p,
            None => {
                return Err(ArchiveError::BadArchiveStructure(
                    "Archive too small".to_owned(),
                ))
            }
        };

        //let mut pos = file_length - 4;
        let search_upper_bound =
            file_length.saturating_sub(END_OF_CENTRAL_DIRECTORY_SIZE + u16::MAX as u64);

        loop {
            if position < search_upper_bound {
                return Err(ArchiveError::BadArchiveStructure(
                    "CENTRAL_DIRECTORY_END_SIGNATURE Not found".to_owned(),
                ));
            }
            /*             println!(
                "position {} >= search_upper_bound {}",
                position, search_upper_bound
            ); */
            reader.seek(SeekFrom::Start(position))?;

            let val = reader.read_u32::<LittleEndian>()?;

            //println!("val {:0X} ", val);
            if val == CENTRAL_DIRECTORY_END_SIGNATURE {
                let signature = stringify!(CENTRAL_DIRECTORY_END_SIGNATURE);
                println!("{signature} found at {}", position);
                break;
            }
            /*             if reader.read_u32::<LittleEndian>()? == CENTRAL_DIRECTORY_END_SIGNATURE {
                reader.seek(io::SeekFrom::Current(
                    BYTES_BETWEEN_MAGIC_AND_COMMENT_SIZE as i64,
                ))?;
                let cde_start_pos = reader.seek(io::SeekFrom::Start(pos))?;
                return CentralDirectoryEnd::parse(reader).map(|cde| (cde, cde_start_pos));
            }*/
            position = match position.checked_sub(1) {
                Some(p) => p,
                None => {
                    let signature = stringify!(CENTRAL_DIRECTORY_END_SIGNATURE);
                    return Err(ArchiveError::BadArchiveStructure(format!(
                        "Signature {signature} Not found"
                    )));
                }
            };
        }

        let central_end_size: usize = (file_length - position - 4) as usize;
        let mut central_end_buffer: Vec<u8> = vec![0; central_end_size];

        println!(
            "central_end_size {} file_length {} location {}",
            central_end_size, file_length, position
        );
        println!("vec len  {} ", central_end_buffer.len());
        //reader.seek(SeekFrom::Start(pos))?;

        reader.read_exact(&mut central_end_buffer)?;

        let central_directory_end = Self::read_cental_directory_end(&central_end_buffer)?;

        println!("central_directory_end {:#?}", central_directory_end);

        let archive_file_entry = Self::read_central_directory(&central_directory_end, reader)?;

        //println!("archive_file_entry {:#?}", archive_file_entry);
        //println!("archive_file_entry file: {}", archive_file_entry);

        Ok((central_directory_end, archive_file_entry))
    }

    fn read_central_directory(
        central_directory_end: &CentralDirectoryEnd,
        reader: &mut R,
    ) -> Result<Vec<ArchiveFileEntry>, ArchiveError> {
        reader.seek(SeekFrom::Start(
            central_directory_end.offset_of_start_of_central_directory,
        ))?;

        let mut central_directory_buffer: Vec<u8> =
            vec![0; central_directory_end.central_directory_size as usize];

        reader.read_exact(&mut central_directory_buffer)?;

        let mut indexer = ArchiveDescriptorReader::new();
        let mut i = 1u32;
        let mut entries: Vec<ArchiveFileEntry> = Vec::new();
        loop {
            let signature = indexer.read_u32(&central_directory_buffer);

            if signature != CENTRAL_DIRECTORY_ENTRY_SIGNATURE {
                println!(
                    "I got {:0X}, I expect {:0X}",
                    signature, CENTRAL_DIRECTORY_ENTRY_SIGNATURE
                );

                println!("{:X?}", central_directory_buffer);
                return Err(ArchiveError::BadArchiveStructure(
                    "Central directory signature not found!".to_owned(),
                ));
            }

            let version_made_by = indexer.read_u16(&central_directory_buffer); // Version made by.
            let version_needed = indexer.read_u16(&central_directory_buffer); // Version needed to extract.
            let general_purpose_flags = indexer.read_u16(&central_directory_buffer); // General purpose flag (temporary crc and sizes + UTF-8 filename).
            let compression_method = indexer.read_u16(&central_directory_buffer); // Compression method .
            let last_mod_file_time = indexer.read_u16(&central_directory_buffer); // Modification time.
            let last_mod_file_date = indexer.read_u16(&central_directory_buffer); // Modification date.
            let crc32 = indexer.read_u32(&central_directory_buffer); // CRC32.
            let compressed_size = indexer.read_u32(&central_directory_buffer) as u64; // Compressed size.
            let uncompressed_size = indexer.read_u32(&central_directory_buffer) as u64; // Uncompressed size.
            let file_name_len = indexer.read_u16(&central_directory_buffer); // Filename length.
            let extra_field_length = indexer.read_u16(&central_directory_buffer); // Extra field length.
            let file_comment_length = indexer.read_u16(&central_directory_buffer); // File comment length.
            let file_disk_number = indexer.read_u16(&central_directory_buffer); // File's Disk number.
            let internal_file_attributes = indexer.read_u16(&central_directory_buffer); // Internal file attributes.
            let external_file_attributes = indexer.read_u32(&central_directory_buffer); // External file attributes (regular file / rw-r--r--).
            let file_info_offset = indexer.read_u32(&central_directory_buffer) as u64;
            let file_name_as_bytes =
                indexer.read_bytes(&central_directory_buffer, file_name_len as usize);

            let compressor = CompressionMethod::from_compression_method(compression_method)?;

            let mut archive_file_entry = ArchiveFileEntry {
                version_made_by,
                minimum_version_needed_to_extract: version_needed,
                general_purpose_flags,
                compression_method,
                last_mod_file_time,
                last_mod_file_date,
                crc32,
                compressed_size,
                uncompressed_size,
                file_name_len,
                extra_field_length,
                file_name_as_bytes,
                offset: file_info_offset,
                compressor,
                internal_file_attributes,
                external_file_attributes,
                file_disk_number: file_disk_number as u32,
                extra_fields: Vec::new(),
                file_comment: None,
                has_zip64_extra_field: false,
            };

            if extra_field_length != 0 {
                //TODO avoid copy
                let extra_field_as_bytes =
                    indexer.read_bytes(&central_directory_buffer, extra_field_length as usize);

                parse_extra_fields(extra_field_as_bytes, &mut archive_file_entry);
            }

            if file_comment_length != 0 {
                let file_comment_as_bytes =
                    indexer.read_bytes(&central_directory_buffer, file_comment_length as usize);

                archive_file_entry.file_comment = Some(file_comment_as_bytes)
            }

            println!("File entry info: {:#?}", archive_file_entry);
            entries.push(archive_file_entry);

            println!("Parsed entry: {}", i);

            println!("-------------------------------------------");
            println!("index {}", indexer.get_index());

            i += 1;
            if indexer.get_index() + CENTRAL_DIRECTORY_ENTRY_BASE_SIZE as usize
                >= central_directory_end.central_directory_size as usize
            {
                break;
            }
        }
        Ok(entries)
    }

    fn read_cental_directory_end(stream: &[u8]) -> Result<CentralDirectoryEnd, ArchiveError> {
        let mut indexer = ArchiveDescriptorReader::new();

        //let _signature = indexer.read_u32(stream);
        let disk_number = indexer.read_u16(stream) as u32;
        let disk_with_central_directory = indexer.read_u16(stream) as u32;
        let total_number_of_entries_on_this_disk = indexer.read_u16(stream) as u64;
        let total_number_of_entries_in_the_central_directory = indexer.read_u16(stream);
        let central_directory_size = indexer.read_u32(stream);
        let offset_of_start_of_central_directory = indexer.read_u32(stream);
        let zip_file_comment_length = indexer.read_u16(stream);

        let archive_comment = indexer.read_bytes(stream, zip_file_comment_length as usize);

        let central_directory_end = CentralDirectoryEnd {
            number_of_this_disk: disk_number,
            number_of_the_disk_with_central_directory: disk_with_central_directory,
            total_number_of_entries_on_this_disk,
            total_number_of_entries_in_the_central_directory:
                total_number_of_entries_in_the_central_directory as u64,
            central_directory_size: central_directory_size as u64,
            offset_of_start_of_central_directory: offset_of_start_of_central_directory as u64,
            archive_comment: Some(archive_comment),
            z64ecdl_relative_offset_of_the_zip64_end_of_central_directory_record: 0,
            z64ecdl_total_number_of_disks: 1,
            z64ecdl_number_of_the_disk_with_the_start_of_the_zip64_end_of_central_directory: 0,
        };

        Ok(central_directory_end)
    }
}

fn parse_extra_fields(
    extra_field_as_bytes: Vec<u8>,
    archive_file_entry: &mut ArchiveFileEntry,
) -> Vec<Box<dyn ExtraField>> {
    let mut indexer = ArchiveDescriptorReader::new();
    let extra_fields = Vec::with_capacity(10);

    while indexer.get_index() + 4 <= extra_field_as_bytes.len() {
        let extra_field_header_id = indexer.read_u16(&extra_field_as_bytes);
        let extra_field_data_size = indexer.read_u16(&extra_field_as_bytes);

        let extra_field: Arc<dyn ExtraField> = match extra_field_header_id {
            ExtraFieldZIP64ExtendedInformation::HEADER_ID => {
                let ef = ExtraFieldZIP64ExtendedInformation::parse_extra_field(
                    &mut indexer,
                    &extra_field_as_bytes,
                    extra_field_data_size,
                    archive_file_entry,
                );
                Arc::new(ef)
            }
            ExtraFieldExtendedTimestamp::HEADER_ID => {
                let ef = ExtraFieldExtendedTimestamp::parse_extra_field(
                    &mut indexer,
                    &extra_field_as_bytes,
                    extra_field_data_size,
                );

                Arc::new(ef)
            }
            _ => {
                let ef = ExtraFieldUnknown::parse_extra_field(
                    &mut indexer,
                    &extra_field_as_bytes,
                    extra_field_data_size,
                    extra_field_header_id,
                );
                Arc::new(ef)
            }
        };

        archive_file_entry.extra_fields.push(extra_field);
    }

    extra_fields
}

impl<R: Read + Seek> Debug for ArchiveReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchiveReader")
            .field("file_entries", &self.file_entries)
            .finish()
    }
}

impl<R: Read + Seek> Display for ArchiveReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Archive:  xxxxxxxxxx.zip")?;

        if let Some(archive_comment) = &self.central_directory_end.archive_comment {
            writeln!(
                f,
                "The zipfile comment is {} bytes long and contains the following text:",
                archive_comment.len()
            )?;
            writeln!(
                f,
                "======================== zipfile comment begins =========================="
            )?;
            writeln!(f, "{}", String::from_utf8_lossy(archive_comment))?;
            writeln!(
                f,
                "========================= zipfile comment ends ==========================="
            )?;
        } else {
            writeln!(f, "There is no zipfile comment.")?;
        }
        writeln!(f)?;
        writeln!(f, "End-of-central-directory record:")?;
        writeln!(f, "-------------------------------")?;
        writeln!(f)?;

        writeln!(f, "{:?}", self.central_directory_end)?;

        let mut i = 1u32;
        for entry in &self.file_entries {
            writeln!(f, "Central directory entry #{}", i)?;
            writeln!(f, "---------------------------")?;
            writeln!(f)?;
            writeln!(f, "{}", entry)?;

            i += 1;
        }

        writeln!(f, "-- END --")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use crate::error::ArchiveError;

    use super::ArchiveReader;

    #[test]
    fn test_mem_dump_rust_zip_lib_lzma() -> Result<(), ArchiveError> {
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

        let buff = Cursor::new(vec);
        ArchiveReader::new(buff)?;
        Ok(())
    }
}
