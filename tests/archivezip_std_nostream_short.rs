use archflow::compress::std::archive::ZipArchive;
use archflow::error::ArchiveError;
use archflow::{compress::FileOptions, compression::CompressionMethod};

mod common;
use common::out_file_name;
use common::std::create_new_clean_file;
const TEST_ID: &str = "short_nostream";
const FILE_TO_COMPRESS: &str = "ex.txt";

fn compress_file(compressor: CompressionMethod, out_file_name: &str) -> Result<(), ArchiveError> {
    let file = create_new_clean_file(out_file_name);

    let mut archive = ZipArchive::new(file);

    let mut in_file = b"example".as_ref();
    let options = FileOptions::default().compression_method(compressor);
    archive
        .append(FILE_TO_COMPRESS, &options, &mut in_file)
        .unwrap();

    let archive_size = archive.finalize().unwrap();
    println!("file {:?} archive size = {:?}", out_file_name, archive_size);
    Ok(())
}

#[test]
fn archive_structure_compress_store() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Store();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name)?;
    Ok(())
}

#[test]
fn archive_structure_zlib_deflate_tokio() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Deflate();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name)?;
    Ok(())
}

#[test]
fn archive_structure_compress_bzip() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::BZip2();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name)?;
    Ok(())
}

#[test]
fn archive_structure_compress_zstd() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Zstd();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name)?;
    Ok(())
}

#[test]
fn archive_structure_compress_xz() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Xz();
    let out_file_name = out_file_name(compressor, TEST_ID);

    compress_file(compressor, &out_file_name)?;
    Ok(())
}
