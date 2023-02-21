use crate::constants::{
    CENTRAL_DIRECTORY_ENTRY_BASE_SIZE, DESCRIPTOR_SIZE, END_OF_CENTRAL_DIRECTORY_SIZE,
    FILE_HEADER_BASE_SIZE,
};

/// Calculate the size that an archive could be based on the names and sizes of files.
///
/// ## Example
///
/// ```no_run
///
/// use zipstream::archive_size;
///
/// assert_eq!(
///     archive_size([
///         ("file1.txt", b"hello\n".len()),
///         ("file2.txt", b"world\n".len()),
///     ]),
///     254,
/// );
/// ```
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
