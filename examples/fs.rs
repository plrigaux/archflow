use archflow::{
    compress::tokio::archive::ZipArchive, compress::FileOptions, compression::CompressionMethod,
    error::ArchiveError,
};

use tokio::fs::File;

#[tokio::main]
async fn main() -> Result<(), ArchiveError> {
    let file = File::create("archive.zip").await.unwrap();

    let options = FileOptions::default().compression_method(CompressionMethod::Deflate());

    let mut archive = ZipArchive::new_streamable(file);

    archive
        .append("file1.txt", &options, &mut b"hello\n".as_ref())
        .await?;

    let options = options.compression_method(CompressionMethod::Store());
    archive
        .append("file2.txt", &options, &mut b"world\n".as_ref())
        .await?;

    archive.finalize().await?;

    Ok(())
}
