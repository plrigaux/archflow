use zipstream::{
    archive::{Archive, FileDateTime},
    compression::{self, Compressor},
};

mod common;
use common::create_new_clean_file;

#[tokio::test]
async fn archive_structure_compress_tokio_zlib_file1() -> Result<(), std::io::Error> {
    let file = create_new_clean_file("test_zlib_tokio2.zip").await;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file_no_extend(
            "file1.txt",
            FileDateTime::now(),
            Compressor::Deflated(),
            &mut f,
        )
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_flate2_zlib_file1() -> Result<(), std::io::Error> {
    let file = create_new_clean_file("test_zlib_flate2.zip").await;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file_no_extend(
            "file1.txt",
            FileDateTime::now(),
            compression::Compressor::DeflatedFate2(),
            &mut f,
        )
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_zup_on_file2() -> Result<(), std::io::Error> {
    let file = create_new_clean_file("test_flat2.zip").await;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file_no_extend(
            "file1.txt",
            FileDateTime::now(),
            Compressor::Store(),
            &mut f,
        )
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_bzip_file1() -> Result<(), std::io::Error> {
    let file = create_new_clean_file("test_bzip1.zip").await;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file(
            "file1.txt",
            FileDateTime::now(),
            Compressor::BZip2(),
            &mut f,
        )
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_xz_file1() -> Result<(), std::io::Error> {
    let file = create_new_clean_file("test_xz2.zip").await;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file_no_extend("file1.txt", FileDateTime::now(), Compressor::Xz(), &mut f)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_zstd_file1() -> Result<(), std::io::Error> {
    let file = create_new_clean_file("test_zstd2.zip").await;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file_no_extend(
            "file1.txt",
            FileDateTime::now(),
            Compressor::BZip2(),
            &mut f,
        )
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}
