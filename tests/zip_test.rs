use std::io::Read;
use std::path::Path;
use std::{fs::File, io::Write};
use zip::{write::FileOptions, DateTime, ZipWriter};

fn create_new_clean_file_std(file_name: &str) -> std::fs::File {
    let dir_prefix = "/tmp/zipstream";
    let out_dir = Path::new(dir_prefix);
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir).unwrap_or_else(|error| {
            panic!("creating dir {:?} failed, because {:?}", dir_prefix, error);
        })
    }

    let out_path = out_dir.join(file_name);

    if out_path.exists() {
        std::fs::remove_file(&out_path).unwrap_or_else(|error| {
            panic!("deleting file {:?} failed, because {:?}", &out_path, error);
        });
    }
    std::fs::File::create(&out_path).unwrap_or_else(|error| {
        panic!("creating file {:?} failed, because {:?}", &out_path, error);
    })
}

const FILE_TO_COMPRESS: &str = "short_text_file.txt";

#[test]
fn store_test() -> Result<(), std::io::Error> {
    base_test("test_zrust_store.zip", zip::CompressionMethod::Stored)
}

#[test]
fn zip_test() -> Result<(), std::io::Error> {
    base_test("test_zrust_zip.zip", zip::CompressionMethod::Deflated)
}

#[test]
fn bzip_test() -> Result<(), std::io::Error> {
    base_test("test_zrust_bzip.zip", zip::CompressionMethod::Bzip2)
}

#[test]
fn zstd_test() -> Result<(), std::io::Error> {
    base_test("test_zrust_zstd.zip", zip::CompressionMethod::Zstd)
}

fn base_test(
    out_file_name: &str,
    compression_method: zip::CompressionMethod,
) -> Result<(), std::io::Error> {
    let file = create_new_clean_file_std(out_file_name);

    let mut zip = ZipWriter::new(file);

    //zip.add_directory("test/", Default::default())?;

    let options = FileOptions::default()
        .compression_method(compression_method)
        .last_modified_time(DateTime::default());

    zip.start_file(FILE_TO_COMPRESS, options)?;

    let path = Path::new("tests").join(FILE_TO_COMPRESS);

    let mut file_to_compress = File::open(path)?;

    let mut buffer = Vec::new();
    file_to_compress.read_to_end(&mut buffer)?;
    zip.write_all(&buffer)?;

    zip.finish()?;

    Ok(())
}

#[test]
fn str_deflate_test() -> Result<(), std::io::Error> {
    let compression_method = zip::CompressionMethod::Zstd;
    let out_file_name = "test_deflate_str.zip";

    let file = create_new_clean_file_std(out_file_name);

    let mut zip = ZipWriter::new(file);

    //zip.add_directory("test/", Default::default())?;

    let options = FileOptions::default()
        .compression_method(compression_method)
        .last_modified_time(DateTime::default());

    zip.start_file("example.txt", options)?;
    let data = b"Example";
    zip.write_all(data.as_ref())?;

    zip.finish()?;

    Ok(())
}
