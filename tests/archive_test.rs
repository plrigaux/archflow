use std::io::Cursor;
use std::path::Path;
use zipstream::{
    archive::{Archive, FileDateTime},
    tools::archive_size,
};
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

#[tokio::test]
async fn archive_structure() {
    let mut archive = Archive::new(Vec::new());
    archive
        .append(
            "file1.txt".to_owned(),
            FileDateTime::now(),
            &mut Cursor::new(b"hello\n".to_vec()),
        )
        .await
        .unwrap();
    archive
        .append(
            "file2.txt".to_owned(),
            FileDateTime::now(),
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
                .into_iter()
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
        .appendzip("file1.txt".to_owned(), FileDateTime::now(), &mut f)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size())
    //let data = archive.finalize().await.unwrap();
}

#[tokio::test]
async fn archive_structure_compress_zlib_file1() -> Result<(), std::io::Error> {
    let out_path = Path::new("/tmp/test_compress1.zip");

    if out_path.exists() {
        tokio::fs::remove_file(out_path).await?;
    }
    let file = tokio::fs::File::create(out_path).await?;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .appendzip("file1.txt".to_owned(), FileDateTime::now(), &mut f)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_zup_on_file2() -> Result<(), std::io::Error> {
    println!("ASDFASDFASDFASDF");
    let out_path = Path::new("/tmp/test_flat1.zip");

    if out_path.exists() {
        tokio::fs::remove_file(out_path).await?;
    }
    let file = tokio::fs::File::create(out_path).await?;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append("file1.txt".to_owned(), FileDateTime::now(), &mut f)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}

#[tokio::test]
async fn archive_structure_compress_bzip_file1() -> Result<(), std::io::Error> {
    println!("ASDFASDFASDFASDF");
    let out_path = Path::new("/tmp/test_bzip1.zip");

    if out_path.exists() {
        tokio::fs::remove_file(out_path).await?;
    }
    let file = tokio::fs::File::create(out_path).await?;

    let mut archive = Archive::new(file);

    let mut f = tokio::fs::File::open("tests/file1.txt").await.unwrap();

    archive
        .append_bzip("file1.txt".to_owned(), FileDateTime::now(), &mut f)
        .await
        .unwrap();

    archive.finalize().await.unwrap();
    println!("archive size = {:?}", archive.get_archive_size());
    //let data = archive.finalize().await.unwrap();

    Ok(())
}
