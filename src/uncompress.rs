use crate::archive_common::{ArchiveDescriptorReader, CentralDirectoryEnd};
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
        let mut position = file_length - END_OF_CENTRAL_DIRECTORY_SIZE;
        //let mut pos = file_length - 4;
        let search_upper_bound =
            file_length.saturating_sub(END_OF_CENTRAL_DIRECTORY_SIZE + u16::MAX as u64);

        loop {
            if position < search_upper_bound {
                return Err(ArchiveError::BadArchiveStructure(
                    "CENTRAL_DIRECTORY_END_SIGNATURE Not found".to_owned(),
                ));
            }
            println!(
                "position {} >= search_upper_bound {}",
                position, search_upper_bound
            );
            reader.seek(SeekFrom::Start(position))?;

            let val = reader.read_u32::<LittleEndian>()?;

            println!("val {:0X} ", val);
            if val == CENTRAL_DIRECTORY_END_SIGNATURE {
                println!("CENTRAL_DIRECTORY_END_SIGNATURE found at {}", position);
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
                    return Err(ArchiveError::BadArchiveStructure(
                        "CENTRAL_DIRECTORY_END_SIGNATURE Not found".to_owned(),
                    ))
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
            central_directory_end.offset_of_start_of_central_directory as u64,
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
            let file_info_offset = indexer.read_u32(&central_directory_buffer);
            let file_name_as_bytes =
                indexer.read_bytes(&central_directory_buffer, file_name_len as usize);

            let compressor = CompressionMethod::from_compression_method(compression_method)?;

            let file_comment = if file_comment_length != 0 {
                let file_comment_as_bytes =
                    indexer.read_bytes(&central_directory_buffer, file_comment_length as usize);

                Some(file_comment_as_bytes)
            } else {
                None
            };

            let archive_file_entry = ArchiveFileEntry {
                version_made_by,
                version_needed,
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
                file_disk_number,
                file_comment,
            };

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
        let disk_number = indexer.read_u16(stream);
        let disk_with_central_directory = indexer.read_u16(stream);
        let total_number_of_entries_on_this_disk = indexer.read_u16(stream);
        let total_number_of_entries = indexer.read_u16(stream);
        let central_directory_size = indexer.read_u32(stream);
        let offset_of_start_of_central_directory = indexer.read_u32(stream);
        let zip_file_comment_length = indexer.read_u16(stream);

        let archive_comment = indexer.read_bytes(stream, zip_file_comment_length as usize);

        let central_directory_end = CentralDirectoryEnd {
            disk_number,
            disk_with_central_directory,
            total_number_of_entries_on_this_disk,
            total_number_of_entries,
            central_directory_size,
            offset_of_start_of_central_directory,
            archive_comment: Some(archive_comment),
        };

        Ok(central_directory_end)
    }
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
        /*         Archive:  test_multiple.zip
        There is no zipfile comment.

        End-of-central-directory record:
        -------------------------------

          Zip archive file size:                      3156 (0000000000000C54h)
          Actual end-cent-dir record offset:          3134 (0000000000000C3Eh)
          Expected end-cent-dir record offset:        3134 (0000000000000C3Eh)
          (based on the length of the central directory and its expected offset)

          This zipfile constitutes the sole disk of a single-part archive; its
          central directory contains 3 entries.
          The central directory is 185 (00000000000000B9h) bytes long,
          and its (expected) offset in bytes from the beginning of the zipfile
          is 2949 (0000000000000B85h).
         */

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
    use std::{fs::File, io::Cursor, path::Path};

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
    #[test]
    fn test_file_rust_zip_lib_lzma() -> Result<(), ArchiveError> {
        let p = Path::new("res_test/outi2.zip");
        let f = File::open(p)?;
        ArchiveReader::new(f)?;
        Ok(())
    }
}
