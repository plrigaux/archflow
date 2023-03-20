use std::path::Path;

use archflow::{
    compress::tokio::archive::ZipArchive, compress::FileOptions, compression::CompressionMethod,
};
mod common;
use common::out_file_name;
use common::tokio::create_new_clean_file;

const TEST_ID: &str = "1";
const FILE_TO_COMPRESS: &str = "file1.txt";

async fn compress_file(compressor: CompressionMethod, out_file_name: &str) {
    let file = create_new_clean_file(out_file_name).await;

    let mut archive = ZipArchive::new_streamable(file);

    let path = Path::new("tests/resources").join(FILE_TO_COMPRESS);
    let mut in_file = tokio::fs::File::open(path).await.unwrap();

    let options = FileOptions::default().compression_method(compressor);
    archive
        .append("file1.txt", &options, &mut in_file)
        .await
        .unwrap();

    let results = archive.finalize().await.unwrap();
    println!("file {:?} archive size = {:?}", out_file_name, results.0);
    //let data = archive.finalize().await.unwrap();
}

#[tokio::test]
async fn archive_structure_compress_store() {
    let compressor = CompressionMethod::Store();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_zlib_deflate_tokio() {
    let compressor = CompressionMethod::Deflate();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_bzip() {
    let compressor = CompressionMethod::BZip2();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_zstd() {
    let compressor = CompressionMethod::Zstd();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_xz() {
    let compressor = CompressionMethod::Xz();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await;
}
