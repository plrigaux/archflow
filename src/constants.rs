use std::mem::size_of;

pub const FILE_HEADER_BASE_SIZE: usize = 7 * size_of::<u16>() + 4 * size_of::<u32>();
pub const DESCRIPTOR_SIZE: usize = 4 * size_of::<u32>();
pub const CENTRAL_DIRECTORY_ENTRY_BASE_SIZE: usize = 11 * size_of::<u16>() + 6 * size_of::<u32>();
pub const END_OF_CENTRAL_DIRECTORY_SIZE: usize = 5 * size_of::<u16>() + 3 * size_of::<u32>();

pub const CENTRAL_DIRECTORY_END_SIGNATURE: u32 = 0x06054b50;
pub const CENTRAL_DIRECTORY_ENTRY_SIGNATURE: u32 = 0x02014b50;
