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
fn zip_test() -> Result<(), std::io::Error> {
    let file = create_new_clean_file_std("test__rust_zip.zip");

    let mut zip = ZipWriter::new(file);

    //zip.add_directory("test/", Default::default())?;

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(DateTime::default());

    zip.start_file(FILE_TO_COMPRESS, options)?;

    let path = Path::new("tests").join(FILE_TO_COMPRESS);

    let mut f = File::open(&path)?;

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    zip.write_all(&buffer)?;
    buffer.clear();

    zip.finish()?;

    Ok(())
}
