use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, DESCRIPTOR_SIZE, END_OF_CENTRAL_DIRECTORY_SIZE,
    FILE_HEADER_BASE_SIZE,
};

pub fn archive_size<'a, I: IntoIterator<Item = (&'a str, usize)>>(files: I) -> usize {
    files
        .into_iter()
        .map(|(name, size)| {
            FILE_HEADER_BASE_SIZE
                + name.len()
                + size
                + DESCRIPTOR_SIZE
                + CENTRAL_DIRECTORY_ENTRY_BASE_SIZE
                + name.len()
        })
        .sum::<usize>()
        + END_OF_CENTRAL_DIRECTORY_SIZE
}
