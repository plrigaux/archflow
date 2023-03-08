use compstream::{
    archive::FileOptions, compress::tokio::archive::ZipArchiveNoStream,
    compression::CompressionMethod,
};

mod common;
use common::out_file_name;
use common::tokio::create_new_clean_file;
const TEST_ID: &str = "str";
const FILE_TO_COMPRESS: &str = "ex.txt";

async fn compress_file(compressor: CompressionMethod, out_file_name: &str) {
    let file = create_new_clean_file(out_file_name).await;

    let mut archive = ZipArchiveNoStream::new(file);

    let mut in_file = b"example".as_ref();
    let options = FileOptions::default().compression_method(compressor);
    archive
        .append_file(FILE_TO_COMPRESS, &mut in_file, &options)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!(
        "file {:?} archive size = {:?}",
        out_file_name,
        archive.get_archive_size()
    );
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
async fn archive_structure_compress_lzma() {
    let compressor = CompressionMethod::Lzma();
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
