use rill::error::ArchiveError;
use rill::{
    archive::FileOptions, compress::tokio::archive::ZipArchive, compression::CompressionMethod,
};

mod common;
use common::out_file_name;
use common::tokio::create_new_clean_file;
const TEST_ID: &str = "str";
const FILE_TO_COMPRESS: &str = "ex.txt";

async fn compress_file(
    compressor: CompressionMethod,
    out_file_name: &str,
) -> Result<(), ArchiveError> {
    let file = create_new_clean_file(out_file_name).await;

    let mut archive = ZipArchive::new(file);

    let mut in_file = b"example".as_ref();
    let options = FileOptions::default().compression_method(compressor);
    archive
        .append_file(FILE_TO_COMPRESS, &mut in_file, &options)
        .await?;

    let results = archive.finalize().await?;
    println!("file {:?} archive size = {:?}", out_file_name, results.0);

    Ok(())
    //let data = archive.finalize().await.unwrap();
}

#[tokio::test]
async fn archive_structure_compress_store() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Store();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await?;
    Ok(())
}

#[tokio::test]
async fn archive_structure_zlib_deflate_tokio() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Deflate();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await?;
    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_bzip() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::BZip2();
    let out_file_name = out_file_name(compressor, TEST_ID);
    compress_file(compressor, &out_file_name).await?;
    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_lzma() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Lzma();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await?;
    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_zstd() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Zstd();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await?;
    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_xz() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Xz();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name).await?;
    Ok(())
}
