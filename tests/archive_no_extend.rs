use std::path::Path;

use compstream::{
    archive::FileOptions, compress::tokio::archive::ZipArchiveNoStream,
    compression::CompressionMethod,
};
mod common;
use common::create_new_clean_file;
const TEST_ID: &str = "NE";
const FILE_TO_COMPRESS: &str = "short_text_file.txt";

async fn compress_file(compressor: CompressionMethod, out_file_name: &str) {
    let file = create_new_clean_file(out_file_name).await;

    let mut archive = ZipArchiveNoStream::new(file);

    let path = Path::new("tests/resources").join(FILE_TO_COMPRESS);

    let mut in_file = tokio::fs::File::open(&path).await.unwrap();

    let options = FileOptions::default().compression_method(compressor);
    archive
        .append_file(FILE_TO_COMPRESS, &mut in_file, &options)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();
}

#[tokio::test]
async fn archive_structure_compress_store() {
    let compressor = CompressionMethod::Store();
    let out_file_name = ["test_", TEST_ID, "_", &compressor.to_string(), ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_zlib_deflate_tokio() {
    let compressor = CompressionMethod::Deflate();
    let out_file_name = [
        "test_",
        TEST_ID,
        "_",
        &compressor.to_string(),
        "_tokio",
        ".zip",
    ]
    .join("");

    compress_file(compressor, &out_file_name).await;
}

/* #[tokio::test]
async fn archive_structure_zlib_deflate_flate2() {
    let compressor = Compressor::DeflateFate2();
    let out_file_name = [
        "test_",
        TEST_ID,
        "_",
        &compressor.to_string(),
        "_flate",
        ".zip",
    ]
    .join("");

    compress_file(compressor, &out_file_name).await;
} */

#[tokio::test]
async fn archive_structure_compress_bzip() {
    let compressor = CompressionMethod::BZip2();
    let out_file_name = ["test_", TEST_ID, "_", &compressor.to_string(), ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_lzma() {
    let compressor = CompressionMethod::Lzma();
    let out_file_name = ["test_", TEST_ID, "_", &compressor.to_string(), ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_zstd() {
    let compressor = CompressionMethod::Zstd();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_xz() {
    let compressor = CompressionMethod::Xz();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}
