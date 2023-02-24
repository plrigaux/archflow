use zipstream::{archive::Archive, compression::Compressor, types::FileDateTime};

mod common;
use common::create_new_clean_file;
const TEST_ID: &str = "NE";

async fn compress_file(compressor: Compressor, out_file_name: &str) {
    let file = create_new_clean_file(out_file_name).await;

    let mut archive = Archive::new(file);

    let mut in_file = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file_no_extend("file1.txt", FileDateTime::Zero, compressor, &mut in_file)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();
}

#[tokio::test]
async fn archive_structure_compress_tokio_deflate() {
    let compressor = Compressor::Deflated();
    let out_file_name = ["test_", &compressor.to_string(), "_tokio", TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_store() {
    let compressor = Compressor::Store();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_zlib_flate() {
    let compressor = Compressor::DeflatedFate2();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, "_flate", ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_bzip_file1() {
    let compressor = Compressor::BZip2();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_xz_file1() {
    let compressor = Compressor::Xz();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_zstd_file1() {
    let compressor = Compressor::Zstd();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}
