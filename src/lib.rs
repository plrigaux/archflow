//! A library for creating ZIP archives in one pass. This is useful when when it is not possible to
//! *seek* in the output such as stdout or a data stream.
//!
//! ZIP is an archive file format that supports lossless data compression. A ZIP file may contain one
//! or more files or directories that may have been compressed. The ZIP file format permits a number
//! of compression algorithms, though DEFLATE is the most common. This format was originally created
//! in 1989 and was first implemented in PKWARE, Inc.'s PKZIP utility
//!
//!
//! The current implementation is based on
//!
//! [PKWARE's APPNOTE.TXT v6.3.10](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT)
//!
//!```rust
//! use archflow::{
//! archive::FileOptions, compress::tokio::archive::ZipArchive, compression::CompressionMethod,
//! error::ArchiveError,
//! };
//!
//! use tokio::fs::File;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ArchiveError> {
//! let file = File::create("archive.zip").await?;
//!
//! let options = FileOptions::default().compression_method(CompressionMethod::Deflate());
//!
//! let mut archive = ZipArchive::new_streamable(file);
//! archive.append_file("file1.txt", &mut b"hello\n".as_ref(), &options).await?;
//!
//! let options = options.compression_method(CompressionMethod::Store());
//! archive.append_file("file2.txt", &mut b"world\n".as_ref(), &options).await?;
//!
//! archive.finalize().await?;Ok(())
//!
//! }
//!```

pub mod archive;
mod constants;

mod archive_common;
pub mod compress;
pub mod compression;
pub mod error;
pub mod tools;
pub mod types;
pub mod uncompress;
