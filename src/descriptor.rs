use std::str;
use std::u32;
use std::u8;

use crate::compression::Compressor;
use crate::types::ArchiveFileEntry;

pub struct ArchiveDescriptor {
    buffer: Vec<u8>,
}

impl ArchiveDescriptor {
    pub fn new(capacity: usize) -> ArchiveDescriptor {
        ArchiveDescriptor {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn write_u16(&mut self, val: u16) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_u32(&mut self, val: u32) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn write_str(&mut self, val: &str) {
        self.write_bytes(val.as_bytes());
    }

    pub fn write_bytes(&mut self, val: &[u8]) {
        self.buffer.extend_from_slice(val);
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

    pub fn read_file_descriptor(stream: &[u8]) -> ArchiveFileEntry {
        let mut indexer = ArchiveDescriptorReader::new();

        let _signature = indexer.read_u32(stream);
        let version_needed = indexer.read_u16(stream);
        let general_purpose_flags = indexer.read_u16(stream);
        let compression_method = indexer.read_u16(stream);
        let time = indexer.read_u16(stream);
        let date = indexer.read_u16(stream);
        let crc = indexer.read_u32(stream);
        let compressed_size = indexer.read_u32(stream);
        let uncompressed_size = indexer.read_u32(stream);
        let file_name_len = indexer.read_u16(stream);
        let extra_field_length = indexer.read_u16(stream);
        let file_name = indexer.read_utf8_string(stream, file_name_len as usize);

        let file_name_as_bytes = file_name.as_bytes().to_owned();

        ArchiveFileEntry {
            version_needed,
            general_purpose_flags,
            last_mod_file_time: time,
            last_mod_file_date: date,
            crc,
            compressed_size,
            uncompressed_size,
            file_name_len,
            extra_field_length,
            file_name_as_bytes,
            offset: 0,

            compression_method,
            compressor: Compressor::from_compression_method(compression_method),
        }
    }
}

struct ArchiveDescriptorReader {
    index: usize,
}

const U_32_LEN: usize = ::std::mem::size_of::<u32>();
const U_16_LEN: usize = ::std::mem::size_of::<u16>();

impl ArchiveDescriptorReader {
    fn new() -> ArchiveDescriptorReader {
        ArchiveDescriptorReader { index: 0 }
    }

    fn read_u32(&mut self, stream: &[u8]) -> u32 {
        let upper_bound = self.index + U_32_LEN;

        let read: [u8; U_32_LEN] = stream[self.index..upper_bound].try_into().unwrap();
        let value = u32::from_le_bytes(read);

        self.index = upper_bound;

        println!("read_u32 value: {:} new index {:}", value, self.index);

        value
    }

    fn read_u16(&mut self, stream: &[u8]) -> u16 {
        let upper_bound = self.index + U_16_LEN;
        let read: [u8; U_16_LEN] = stream[self.index..upper_bound].try_into().unwrap();
        let value = u16::from_le_bytes(read);

        self.index = upper_bound;

        println!("read_u16 value: {:?} new index {:}", value, self.index);

        value
    }

    fn read_utf8_string(&mut self, stream: &[u8], string_len: usize) -> String {
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

        println!("read_u16 value: {:?} new index {:}", value, self.index);

        value
    }
}

#[cfg(test)]
mod test {
    use crate::{compression::Compressor, constants::LOCAL_FILE_HEADER_SIGNATURE};

    use super::ArchiveDescriptor;

    #[test]
    fn test_write_file_header() {
        let version_needed = Compressor::Deflated().version_needed();
        let compression_method = Compressor::Deflated().compression_method();
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
}

#[test]
fn test_mem_dump() {
    let vec: Vec<u8> = vec![
        0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x1D,
        0x85, 0xB7, 0xB3, 0xB9, 0x36, 0x00, 0x00, 0xDF, 0xE0, 0x3E, 0x00, 0x09, 0x00, 0x00, 0x00,
        0x66, 0x69, 0x6C, 0x65, 0x31, 0x2E, 0x74, 0x78, 0x74, 0xED, 0xCD, 0x39, 0x11, 0x00, 0x20,
        0x0C, 0x00, 0xB0, 0x1D, 0x35, 0x14, 0xCA, 0xE7, 0xDF, 0x18, 0x2A, 0x18, 0xB8, 0xCB, 0x96,
        0x2D, 0xD1, 0x7A, 0x8E, 0xB9, 0xF6, 0xA9, 0xF1, 0x4C, 0x25, 0x24, 0x12, 0x89, 0x44, 0x22,
        0x91, 0x48, 0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48, 0x24, 0x12, 0x89, 0x44, 0x22, 0x91,
        0x48, 0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48, 0x24, 0x12, 0x89, 0x44, 0x22, 0x91, 0x48,
        0x24, 0x12, 0x89, 0x44, 0x22,
    ];

    let entry = ArchiveDescriptor::read_file_descriptor(&vec);

    print!("{:#?}", entry)
}

#[test]
fn test_mem_dump2() {
    let vec: Vec<u8> = vec![
        0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1d,
        0x85, 0xb7, 0xb3, 0xc6, 0x36, 0x00, 0x00, 0xdf, 0xe0, 0x3e, 0x00, 0x09, 0x00, 0x00, 0x00,
        0x66, 0x69, 0x6c, 0x65, 0x31, 0x2e, 0x74, 0x78, 0x74, 0x78, 0x9c, 0xec, 0xcd, 0x39, 0x11,
        0x00, 0x20, 0x0c,
    ];
    let entry = ArchiveDescriptor::read_file_descriptor(&vec);

    print!("{:#?}", entry)
}

#[test]
fn test_mem_dump3() {
    let vec: Vec<u8> = vec![
        0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1d,
        0x85, 0xb7, 0xb3, 0xc6, 0x36, 0x00, 0x00, 0xdf, 0xe0, 0x3e, 0x00, 0x09, 0x00, 0x00, 0x00,
        0x66, 0x69, 0x6c, 0x65, 0x31, 0x2e, 0x74, 0x78, 0x74, 0x78, 0x9c, 0xec, 0xcd, 0x39, 0x11,
        0x00, 0x20, 0x0c,
    ];
    let entry = ArchiveDescriptor::read_file_descriptor(&vec);

    println!("{:#?}", entry);

    println!("\nFile descriptor\n{}", entry);
}
