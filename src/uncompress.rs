use crate::archive_common::{ArchiveDescriptorReader, CentralDirectoryEnd};
use crate::compression::CompressionMethod;
use crate::constants::CENTRAL_DIRECTORY_ENTRY_SIGNATURE;
use crate::types::ArchiveFileEntry;
use crate::{
    constants::{CENTRAL_DIRECTORY_END_SIGNATURE, END_OF_CENTRAL_DIRECTORY_SIZE},
    error::ArchiveError,
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

pub struct ArchiveReader<R>
where
    R: Read + Seek,
{
    #[allow(dead_code)]
    reader: R,
}

impl<R: Read + Seek> ArchiveReader<R> {
    pub fn new(mut reader: R) -> Result<ArchiveReader<R>, ArchiveError> {
        Self::parse(&mut reader)?;

        let ar = ArchiveReader { reader };
        Ok(ar)
    }

    fn parse(reader: &mut R) -> Result<(), ArchiveError> {
        //find central dir end

        let file_length = reader.seek(SeekFrom::End(0))?;
        let mut pos = file_length - END_OF_CENTRAL_DIRECTORY_SIZE;
        //let mut pos = file_length - 4;
        let search_upper_bound =
            file_length.saturating_sub(END_OF_CENTRAL_DIRECTORY_SIZE + u16::MAX as u64);

        loop {
            if pos < search_upper_bound {
                return Err(ArchiveError::BadArchiveStructure(
                    "CENTRAL_DIRECTORY_END_SIGNATURE Not found".to_owned(),
                ));
            }
            println!("pos {} >= search_upper_bound {}", pos, search_upper_bound);
            reader.seek(SeekFrom::Start(pos))?;

            let val = reader.read_u32::<LittleEndian>()?;

            println!("val {:0X} ", val);
            if val == CENTRAL_DIRECTORY_END_SIGNATURE {
                println!("CENTRAL_DIRECTORY_END_SIGNATURE found at {}", pos);
                break;
            }
            /*             if reader.read_u32::<LittleEndian>()? == CENTRAL_DIRECTORY_END_SIGNATURE {
                reader.seek(io::SeekFrom::Current(
                    BYTES_BETWEEN_MAGIC_AND_COMMENT_SIZE as i64,
                ))?;
                let cde_start_pos = reader.seek(io::SeekFrom::Start(pos))?;
                return CentralDirectoryEnd::parse(reader).map(|cde| (cde, cde_start_pos));
            }*/
            pos = match pos.checked_sub(1) {
                Some(p) => p,
                None => {
                    return Err(ArchiveError::BadArchiveStructure(
                        "CENTRAL_DIRECTORY_END_SIGNATURE Not found".to_owned(),
                    ))
                }
            };
        }

        let central_end_size: usize = (file_length - pos - 4) as usize;
        let mut v: Vec<u8> = vec![0; central_end_size];

        println!(
            "central_end_size {} file_length {} pos {}",
            central_end_size, file_length, pos
        );
        println!("vec len  {} ", v.len());
        //reader.seek(SeekFrom::Start(pos))?;

        reader.read_exact(&mut v)?;

        let central_directory_end = Self::read_cental_directory_end(&v)?;

        println!("central_directory_end {:#?}", central_directory_end);

        let archive_file_entry = Self::read_cental_directory(central_directory_end, reader)?;

        //println!("archive_file_entry {:#?}", archive_file_entry);
        println!("archive_file_entry file: {}", archive_file_entry);

        Ok(())
    }

    fn read_cental_directory(
        central_directory_end: CentralDirectoryEnd,
        reader: &mut R,
    ) -> Result<ArchiveFileEntry, ArchiveError> {
        reader.seek(SeekFrom::Start(
            central_directory_end.offset_of_start_of_central_directory as u64,
        ))?;

        let mut central_directory_buffer: Vec<u8> =
            vec![0; central_directory_end.central_directory_size as usize];

        reader.read_exact(&mut central_directory_buffer)?;

        let mut indexer = ArchiveDescriptorReader::new();

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
        let a = ArchiveFileEntry {
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
            file_comment_length,
            file_disk_number,
        };

        Ok(a)
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

        let central_directory_end = CentralDirectoryEnd {
            disk_number,
            disk_with_central_directory,
            total_number_of_entries_on_this_disk,
            total_number_of_entries,
            central_directory_size,
            offset_of_start_of_central_directory,
            zip_file_comment_length,
        };

        Ok(central_directory_end)
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
