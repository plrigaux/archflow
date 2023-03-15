# Compstream


https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT


## Features
- Stream on the fly an archive from multiple AsyncRead objects.
- Single read / seek free implementation (the CRC and file size are calculated while streaming and are sent afterwards).
- [tokio](https://docs.rs/tokio/latest/tokio/io/index.html) `AsyncRead` / `AsyncWrite` compatible. 

Supported compression formats:
 - stored (i.e. none)
 - deflate
 - bzip2
 - zstd
 - xz

## Todos

- implement zip64
- implement some zip features (unix time, file comments, ...)
- add more cargo features like for compressors selection

## Examples

- How to create a zip archive
- How to stream an aschive with Hyper
.
### [File system](examples/fs.rs)

```rust
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
```

### [Hyper](examples/hyper.rs)


## Disclaimer

This implementation is inspired by : 
 - https://github.com/scotow/zipit and
 - https://github.com/zip-rs/zip

