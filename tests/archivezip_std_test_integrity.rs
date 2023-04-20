use std::{fs::File, path::Path};

use archflow::{
    compress::std::archive::ZipArchive, compress::FileOptions, compression::CompressionMethod,
    error::ArchiveError, uncompress::ArchiveReader,
};
mod common;
use common::std::create_new_clean_file;
use common::std::MockReader;

#[test]
fn archive_multiple() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple.zip";

    let path = Path::new("tests/resources/lorem_ipsum.txt");
    let mut in_file = File::open(path)?;

    let out_file = create_new_clean_file(out_file_name);
    let mut archive = ZipArchive::new_streamable(out_file);

    let options = FileOptions::default().compression_method(CompressionMethod::Xz());
    archive.append("file1.txt", &options, &mut in_file)?;

    let mut in_file = File::open(path)?;
    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    archive.append("file2.txt", &options, &mut in_file)?;

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Store())
        .set_file_comment("This is a store file");
    archive.append("file4.txt", &options, &mut b"Some string data".as_ref())?;

    archive.set_archive_comment("This is a comment for the archive, This is a comment for the archive, This is a comment for the archive, This is a comment for the archive");
    let (archive_size, out_file) = archive.finalize()?;

    println!("Archive size {}", archive_size);

    println!("Archive file {:?}", out_file);

    let out_file_path = Path::new("/tmp/archflow/std/test_multiple.zip");
    let out_file = File::open(out_file_path).unwrap();

    let archive_read = ArchiveReader::new(out_file).unwrap();

    println!("{}", archive_read);

    //test files

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        3
    );

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        archive_read.file_entries.len() as u64
    );

    let mut iter = archive_read.file_entries.iter();
    let entry1 = iter.next().unwrap();
    let entry2 = iter.next().unwrap();
    let entry3 = iter.next().unwrap();

    assert_eq!("file1.txt", entry1.get_file_name());
    assert_eq!("file2.txt", entry2.get_file_name());
    assert_eq!("file4.txt", entry3.get_file_name());
    //test time

    //test compression meth

    Ok(())
}

#[test]
fn archive_multiple_mock() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple_mock.zip";

    let out_file = create_new_clean_file(out_file_name);
    let mut archive = ZipArchive::new_streamable(out_file);

    let options = FileOptions::default().compression_method(CompressionMethod::Xz());
    let mut file = MockReader::new(5000);
    archive.append("zeros1.txt", &options, &mut file)?;

    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    let mut file = MockReader::new(5000);
    archive.append("zeros2.txt", &options, &mut file)?;

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Store())
        .set_file_comment("This is a store file");
    archive.append("zeros3.txt", &options, &mut b"Some string data".as_ref())?;

    archive.set_archive_comment("This is a comment for the archive, This is a comment for the archive, This is a comment for the archive, This is a comment for the archive");
    let (archive_size, out_file) = archive.finalize()?;

    println!("Archive size {}", archive_size);

    println!("Archive file {:?}", out_file);

    let out_file_path = Path::new("/tmp/archflow/std/").join(out_file_name);
    let out_file = File::open(out_file_path).unwrap();

    let archive_read = ArchiveReader::new(out_file).unwrap();

    println!("{}", archive_read);

    //test files

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        3
    );

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        archive_read.file_entries.len() as u64
    );

    let mut iter = archive_read.file_entries.iter();
    let entry1 = iter.next().unwrap();
    let entry2 = iter.next().unwrap();
    let entry3 = iter.next().unwrap();

    assert_eq!("zeros1.txt", entry1.get_file_name());
    assert_eq!("zeros2.txt", entry2.get_file_name());
    assert_eq!("zeros3.txt", entry3.get_file_name());
    //test time

    //test compression meth

    Ok(())
}

#[test]
fn archive_multiple_mock_z64() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple_mock_64.zip";

    let out_file = create_new_clean_file(out_file_name);
    let mut archive = ZipArchive::new_streamable(out_file);

    let options = FileOptions::default().compression_method(CompressionMethod::Xz());
    let mut file = MockReader::new(u32::MAX as usize + 10);
    archive.append("zeros1.txt", &options, &mut file)?;

    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    let mut file = MockReader::new(u16::MAX as usize + 10);
    archive.append("zeros2.txt", &options, &mut file)?;

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Store())
        .set_file_comment("This is a store file");
    let mut file = MockReader::new(u8::MAX as usize + 10);
    archive.append("zeros3.txt", &options, &mut file)?;

    archive.set_archive_comment("This is a comment for the archive, This is a comment for the archive, This is a comment for the archive, This is a comment for the archive");
    let (archive_size, out_file) = archive.finalize()?;

    println!("Archive size {}", archive_size);

    println!("Archive file {:?}", out_file);

    let out_file_path = Path::new("/tmp/archflow/std/").join(out_file_name);
    let out_file = File::open(out_file_path).unwrap();

    let archive_read = ArchiveReader::new(out_file).unwrap();

    println!("{}", archive_read);

    //test files

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        3
    );

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        archive_read.file_entries.len() as u64
    );

    let mut iter = archive_read.file_entries.iter();
    let entry1 = iter.next().unwrap();
    let entry2 = iter.next().unwrap();
    let entry3 = iter.next().unwrap();

    assert_eq!("zeros1.txt", entry1.get_file_name());
    assert_eq!("zeros2.txt", entry2.get_file_name());
    assert_eq!("zeros3.txt", entry3.get_file_name());
    //test time

    //test compression meth

    Ok(())
}

#[allow(dead_code)]
fn archive_multiple_mock_z64_read() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple_mock_64.zip";

    let out_file_path = Path::new("/tmp/archflow/std/").join(out_file_name);
    let out_file = File::open(out_file_path).unwrap();

    let archive_read = ArchiveReader::new(out_file).unwrap();

    println!("{}", archive_read);

    //test files

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        3
    );

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        archive_read.file_entries.len() as u64
    );

    let mut iter = archive_read.file_entries.iter();
    let entry1 = iter.next().unwrap();
    let entry2 = iter.next().unwrap();
    let entry3 = iter.next().unwrap();

    assert_eq!("zeros1.txt", entry1.get_file_name());
    assert_eq!("zeros2.txt", entry2.get_file_name());
    assert_eq!("zeros3.txt", entry3.get_file_name());
    //test time

    //test compression meth

    Ok(())
}
