use std::{fs::File, path::Path};

use archflow::error::ArchiveError;
use archflow::{
    archive::FileOptions, compress::std::archive::ZipArchive, compression::CompressionMethod,
};
mod common;
use common::out_file_name;
use common::std::create_new_clean_file;

const TEST_ID: &str = "stream";
const FILE_TO_COMPRESS: &str = "file1.txt";

fn compress_file(compressor: CompressionMethod, out_file_name: &str) -> Result<(), ArchiveError> {
    let file = create_new_clean_file(out_file_name);

    let mut archive = ZipArchive::new_streamable(file);

    let path = Path::new("tests/resources").join(FILE_TO_COMPRESS);
    let mut in_file = File::open(path).unwrap();

    let options = FileOptions::default().compression_method(compressor);
    archive.append_file("file1.txt", &mut in_file, &options)?;

    let archive_size = archive.finalize()?;
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
fn archive_structure_compress_deflate() -> Result<(), ArchiveError> {
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
fn archive_structure_compress_lzma() -> Result<(), ArchiveError> {
    let compressor = CompressionMethod::Lzma();
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

#[test]
fn archive_multiple() -> Result<(), ArchiveError> {
    let out_file_name = "test_multiple.zip";

    let path = Path::new("tests/resources/lorem_ipsum.txt");
    let mut in_file = File::open(path)?;

    let out_file = create_new_clean_file(out_file_name);
    let mut archive = ZipArchive::new_streamable(out_file);

    let options = FileOptions::default().compression_method(CompressionMethod::Xz());
    archive.append_file("file1.txt", &mut in_file, &options)?;

    let mut in_file = File::open(path)?;
    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    archive.append_file("file2.txt", &mut in_file, &options)?;

    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
    archive.append_file("file4.txt", &mut b"Some string data".as_ref(), &options)?;

    archive.finalize()?;

    /*let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
         archive.append_file(
        "file5.txt",
        &mut b"Some string data plus ".as_ref(),
        &options,
    )?; */

    Ok(())
}
