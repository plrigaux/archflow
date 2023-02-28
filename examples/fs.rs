use compstream::{archive::Archive, compression::Compressor, types::FileDateTime};
use std::io::Cursor;
use tokio::fs::File;

#[tokio::main]
async fn main() {
    let file = File::create("archive.zip").await.unwrap();
    let mut archive = Archive::new(file);
    archive
        .append_file(
            "file1.txt",
            FileDateTime::now(),
            Compressor::Store(),
            &mut Cursor::new(b"hello\n"),
        )
        .await
        .unwrap();
    archive
        .append_file(
            "file2.txt",
            FileDateTime::now(),
            Compressor::Store(),
            &mut Cursor::new(b"world\n".to_vec()),
        )
        .await
        .unwrap();
    archive.finalize().await.unwrap();
}
