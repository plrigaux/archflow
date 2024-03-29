use std::path::Path;

use tokio::fs::File;

use archflow::{
    compress::tokio::archive::ZipArchive, compress::FileOptions, compression::CompressionMethod,
    error::ArchiveError, uncompress::ArchiveReader,
};
mod common;

use common::tokio::{create_new_clean_file, get_out_file};

#[tokio::test]
async fn archive_multiple() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple.zip";

    let path = Path::new("tests/resources/lorem_ipsum.txt");
    let mut in_file = File::open(path).await?;

    let out_file = create_new_clean_file(out_file_name).await;
    let mut archive = ZipArchive::new_streamable(out_file);

    let options = FileOptions::default().compression_method(CompressionMethod::Xz());
    archive.append("file1.txt", &options, &mut in_file).await?;

    let mut in_file = File::open(path).await?;
    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    archive.append("file2.txt", &options, &mut in_file).await?;

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Store())
        .set_file_comment("This is a store file");
    archive
        .append("file3.txt", &options, &mut b"Some string data".as_ref())
        .await?;

    //let path = Path::new("tests/resources");
    //let mut in_file = File::open(path).await?;

    let path = Path::new("tests/resources/rust-mascot.png");
    let mut in_file = File::open(path).await?;
    archive.append("mascot.png", &options, &mut in_file).await?;

    archive.append_directory("test_dir", &options).await?;

    archive.set_archive_comment("This is a comment for the archive, This is a comment for the archive, This is a comment for the archive, This is a comment for the archive");
    let (archive_size, out_file) = archive.finalize().await?;

    println!("Archive size {}", archive_size);

    println!("Archive file {:?}", out_file);

    let out_file_path = get_out_file(out_file_name);
    let out_file = std::fs::File::open(out_file_path).unwrap();

    let archive_read = ArchiveReader::new(out_file).unwrap();

    println!("{}", archive_read);

    //test files

    assert_eq!(
        archive_read
            .central_directory_end
            .total_number_of_entries_in_the_central_directory,
        5
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
    let entry4 = iter.next().unwrap();

    assert_eq!("file1.txt", entry1.get_file_name());
    assert_eq!("file2.txt", entry2.get_file_name());
    assert_eq!("file3.txt", entry3.get_file_name());
    assert_eq!("mascot.png", entry4.get_file_name());
    //test time

    assert!(entry3.is_apparently_text_file());
    assert!(!entry4.is_apparently_text_file());
    //test compression meth

    Ok(())
}

#[tokio::test]
async fn archive_multiple_norm() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple_norm.zip";

    let path = Path::new("tests/resources/lorem_ipsum.txt");
    let mut in_file = File::open(path).await?;

    let out_file = create_new_clean_file(out_file_name).await;
    let mut archive = ZipArchive::new(out_file);

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Xz())
        .set_file_comment("file1.txt complessed with xz")
        .time_stamp(None, None, Some(1618854985));
    archive.append("file1.txt", &options, &mut in_file).await?;

    let mut in_file = File::open(path).await?;
    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    archive.append("file2.txt", &options, &mut in_file).await?;

    let options = FileOptions::default()
        .compression_method(CompressionMethod::Store())
        .set_file_comment("This is a store file")
        .time_stamp(Some(1681926985), Some(1681928985), Some(1618854985));

    archive
        .append("file4.txt", &options, &mut b"Some string data".as_ref())
        .await?;

    archive.set_archive_comment("This is a comment for the archive, This is a comment for the archive, This is a comment for the archive, This is a comment for the archive");
    let (archive_size, out_file) = archive.finalize().await?;

    println!("Archive size {}", archive_size);

    println!("Archive file {:?}", out_file);

    let out_file_path = get_out_file(out_file_name);
    let out_file = std::fs::File::open(out_file_path).unwrap();

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
