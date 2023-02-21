use std::io::Read;
use std::path::Path;
use std::{fs::File, io::Write};
use zip::{write::FileOptions, ZipWriter};

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

#[test]
fn zip_test() -> Result<(), std::io::Error> {
    let file = create_new_clean_file_std("test_zip.zip");

    /*     let path = std::path::Path::new(filename);
    let file = std::fs::File::create(path).unwrap(); */

    let mut zip = ZipWriter::new(file);

    //zip.add_directory("test/", Default::default())?;

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    zip.start_file("file1.txt", options)?;
    let mut f = File::open("tests/file1.txt")?;

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    zip.write_all(&buffer)?;
    buffer.clear();

    zip.finish()?;

    Ok(())
}
