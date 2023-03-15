use archflow::{
    archive::FileOptions, compress::tokio::archive::ZipArchive, compression::CompressionMethod,
    error::ArchiveError,
};

use tokio::fs::File;

#[tokio::main]
async fn main() -> Result<(), ArchiveError> {
    let file = File::create("archive.zip").await.unwrap();

    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());

    let mut archive = ZipArchive::new_streamable(file);

    archive
        .append_file("file1.txt", &mut b"hello\n".as_ref(), &options)
        .await?;

    let options = options.compression_method(CompressionMethod::Store());
    archive
        .append_file("file2.txt", &mut b"world\n".as_ref(), &options)
        .await?;

    archive.finalize().await?;

    Ok(())
}
