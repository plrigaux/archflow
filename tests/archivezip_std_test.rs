use std::{fs::File, path::Path};

use compstream::{
    archive::FileOptions, compress::std::archive::ZipArchive, compression::CompressionMethod,
};
mod common;
use common::std::create_new_clean_file;

const TEST_ID: &str = "1";
const FILE_TO_COMPRESS: &str = "file1.txt";

fn compress_file(compressor: CompressionMethod, out_file_name: &str) {
    let file = create_new_clean_file(out_file_name);

    let mut archive = ZipArchive::new(file);

    let path = Path::new("tests/resources").join(FILE_TO_COMPRESS);
    let mut in_file = File::open(path).unwrap();

    let options = FileOptions::default().compression_method(compressor);
    archive
        .append_file("file1.txt", &mut in_file, &options)
        .unwrap();

    archive.finalize().unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().unwrap();
}

#[test]
fn archive_structure_compress_store() {
    let compressor = CompressionMethod::Store();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name);
}

#[test]
fn archive_structure_zlib_deflate_tokio() {
    let compressor = CompressionMethod::Deflate();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name);
}

/* #[test]
 fn archive_structure_zlib_deflate_flate2() {
    let compressor = Compressor::DeflateFate2();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, "_flate", ".zip"].join("");

    compress_file(compressor, &out_file_name);
}
 */
#[test]
fn archive_structure_compress_bzip() {
    let compressor = CompressionMethod::BZip2();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name);
}

#[test]
fn archive_structure_compress_lzma() {
    let compressor = CompressionMethod::Lzma();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name);
}

#[test]
fn archive_structure_compress_zstd() {
    let compressor = CompressionMethod::Zstd();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name);
}

#[test]
fn archive_structure_compress_xz() {
    let compressor = CompressionMethod::Xz();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name);
}
