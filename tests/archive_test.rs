use zipstream::{
    archive::Archive, compression::Compressor, tools::archive_size, types::FileDateTime,
};

mod common;
use common::create_new_clean_file;

const TEST_ID: &str = "1";

#[test]
fn archive_size_test() {
    assert_eq!(
        archive_size([
            ("file1.txt", b"hello\n".len()),
            ("file2.txt", b"world\n".len()),
        ]),
        254,
    );
    assert_eq!(
        archive_size([
            ("file1.txt", b"hello\n".len()),
            ("file2.txt", b"world\n".len()),
            ("file3.txt", b"how are you?\n".len()),
        ]),
        377,
    );
}

/* #[tokio::test]
async fn archive_structure() {
    let mut archive = Archive::new(Vec::new());
    archive
        .append_file(
            "file1.txt",
            FileDateTime::Zero,
            Compressor::Store(),
            &mut Cursor::new(b"hello\n".to_vec()),
        )
        .await
        .unwrap();
    archive
        .append_file(
            "file2.txt",
            FileDateTime::Zero,
            Compressor::Store(),
            &mut Cursor::new(b"world\n".to_vec()),
        )
        .await
        .unwrap();
    archive.finalize().await.unwrap();

    fn match_except_datetime(a1: &[u8], a2: &[u8]) -> bool {
        let datetime_ranges = [
            10..12,
            12..14,
            71..73,
            73..75,
            134..136,
            136..138,
            189..191,
            191..193,
        ];
        let size_ranges = [18..22, 22..26, 79..83, 83..87];
        a1.len() == a2.len()
            && a1
                .iter()
                .zip(a2)
                .enumerate()
                .filter(|(i, _)| {
                    datetime_ranges
                        .iter()
                        .chain(&size_ranges)
                        .all(|range| !range.contains(i))
                })
                .all(|(_, (b1, b2))| b1 == b2)
    }

    let data = archive.retrieve_writer();
    assert!(match_except_datetime(
        &data,
        include_bytes!("timeless_test_archive.zip")
    ));
    assert!(match_except_datetime(
        &data,
        include_bytes!("zip_command_test_archive.zip")
    ));
}

#[tokio::test]
async fn archive_structure_zup() {
    let v = Vec::new();
    let mut archive = Archive::new(v);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file(
            "file1.txt",
            FileDateTime::Zero,
            Compressor::Deflated(),
            &mut f,
        )
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size())
    //let data = archive.finalize().await.unwrap();
}
 */

async fn compress_file(compressor: Compressor, out_file_name: &str) {
    let file = create_new_clean_file(out_file_name).await;

    let mut archive = Archive::new(file);

    let mut in_file = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_file("file1.txt", FileDateTime::Zero, compressor, &mut in_file)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();
}

#[tokio::test]
async fn archive_structure_compress_store() {
    let compressor = Compressor::Store();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_zlib_deflate_tokio() {
    let compressor = Compressor::Deflate();
    let out_file_name = ["test_", &compressor.to_string(), "_tokio", TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_zlib_deflate_flate2() {
    let compressor = Compressor::DeflateFate2();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, "_flate", ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_bzip() {
    let compressor = Compressor::BZip2();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_lzma() {
    let compressor = Compressor::Lzma();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_zstd() {
    let compressor = Compressor::Zstd();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}

#[tokio::test]
async fn archive_structure_compress_xz() {
    let compressor = Compressor::Xz();
    let out_file_name = ["test_", &compressor.to_string(), TEST_ID, ".zip"].join("");

    compress_file(compressor, &out_file_name).await;
}
