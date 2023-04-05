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
//!
//! ## Features
//! - Stream on the fly an archive in one pass (i.e. no seek).
//! - [tokio](https://docs.rs/tokio/latest/tokio/io/index.html) `AsyncRead` / `AsyncWrite` and `AsyncSeek` compatible.
//! - [std::io](https://doc.rust-lang.org/std/io/index.html) `Read` / `Write` and `Seek` compatible
//! - File type detection (text or binary)
//! - Zip64 format (archive size over 4Gb)
//! - Unix time
//!
//! Support the following compression formats:
//! - stored (i.e. none)
//! - deflate
//! - bzip2
//! - zstd
//! - xz
//!
//! ## Cargo features
//! When included in you project, you have the choice to choose the [tokio](https://tokio.rs/) (asynchronous) or standard version.
//!
//! Feature  | Description
//! ---------|------
//! tokio    | To use tokio non blocking API, namely: [tokio::io::AsyncRead], [tokio::io::AsyncWrite] and [tokio::io::AsyncSeek]
//! std      | To use standard API, namely:  [std::io::Read], [std::io::Write] and [std::io::Seek]
//!
//!
//! ## Examples
//! ### [File system](examples/fs.rs)
//!
//! A simple example to create an archive file using [tokio::fs::File]
//!
//!```rust
//! use archflow::{
//! compress::FileOptions, compress::tokio::archive::ZipArchive, compression::CompressionMethod,
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
//! archive.append("file1.txt", &options, &mut b"hello\n".as_ref()).await?;
//!
//! let options = options.compression_method(CompressionMethod::Store());
//! archive.append("file2.txt", &options, &mut b"world\n".as_ref()).await?;
//!
//! archive.finalize().await?;
//!
//! Ok(())
//! }
//!```
//! ### [Actix](examples/actix.rs)
//!
//! Stream a zip archive as a [actix](https://actix.rs/) response:
//!
//!```rust
//! use std::path::Path;
//!
//! use actix_web::http::header::ContentDisposition;
//! use actix_web::{get, App, HttpResponse, HttpServer};
//! use archflow::compress::tokio::archive::ZipArchive;
//! use archflow::compress::FileOptions;
//! use archflow::compression::CompressionMethod;
//! use archflow::types::FileDateTime;
//! use tokio::fs::File;
//! use tokio::io::duplex;
//!
//! use tokio_util::io::ReaderStream;
//!
//! #[get("/zip")]
//! async fn zip_archive() -> HttpResponse {
//!     let (w, r) = duplex(4096);
//!     let options = FileOptions::default()
//!         .last_modified_time(FileDateTime::Now)
//!         .compression_method(CompressionMethod::Deflate());
//!
//!     tokio::spawn(async move {
//!         let mut archive = ZipArchive::new_streamable(w);
//!
//!         let file_path = Path::new("./tests/resources/lorem_ipsum.txt");
//!
//!         let mut file = File::open(file_path).await.unwrap();
//!         archive
//!             .append("ipsum_deflate.txt", &options, &mut file)
//!             .await
//!             .unwrap();
//!
//!         archive
//!             .append("file1.txt", &options, &mut b"world\n".as_ref())
//!             .await
//!             .unwrap();
//!
//!         archive
//!             .append("file2.txt", &options, &mut b"world\n".as_ref())
//!             .await
//!             .unwrap();
//!
//!         let mut f = File::open(file_path).await.unwrap();
//!         let options = options.compression_method(CompressionMethod::BZip2());
//!         archive
//!             .append("ipsum_bz.txt", &options, &mut f)
//!             .await
//!             .unwrap();
//!
//!         let mut f = File::open(file_path).await.unwrap();
//!         let options = options.compression_method(CompressionMethod::Xz());
//!         archive
//!             .append("ipsum_xz.txt", &options, &mut f)
//!             .await
//!             .unwrap();
//!
//!         archive.finalize().await.unwrap();
//!     });
//!
//!     HttpResponse::Ok()
//!         .insert_header(("Content-Type", "application/zip"))
//!         .insert_header(ContentDisposition::attachment("myzip.zip"))
//!         .streaming(ReaderStream::new(r))
//! }
//!
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     let address = "127.0.0.1";
//!     let port = 8081;
//!
//!     println!("Test url is http://{}:{}/zip", address, port);
//!
//!     HttpServer::new(|| App::new().service(zip_archive))
//!         .bind((address, port))?
//!         .run()
//!         .await
//! }
//!
//!```
//!
//! ### [Hyper](examples/hyper.rs)
//!
//! Stream a zip archive as a [hyper](https://hyper.rs/) response:
//!
//!``` rust
//!
//! use archflow::{
//! compress::tokio::archive::ZipArchive, compress::FileOptions, compression::CompressionMethod,
//! types::FileDateTime,
//! };
//! use hyper::service::{make_service_fn, service_fn};
//! use hyper::{header, Body, Request, Response, Server, StatusCode};
//! use tokio::io::duplex;
//! use tokio_util::io::ReaderStream;
//!
//! async fn zip_archive(_req: Request<Body>) -> Result<Response<Body>, hyper::http::Error> {
//!     let (w, r) = duplex(4096);
//!     let options = FileOptions::default()
//!         .compression_method(CompressionMethod::Deflate())
//!         .last_modified_time(FileDateTime::Now);
//!     tokio::spawn(async move {
//!         let mut archive = ZipArchive::new_streamable(w);
//!         archive
//!             .append("file1.txt", &options, &mut b"world\n".as_ref())
//!             .await
//!             .unwrap();
//!         archive
//!             .append("file2.txt", &options, &mut b"world\n".as_ref())
//!             .await
//!             .unwrap();
//!         archive.finalize().await.unwrap();
//!     });
//!
//!     Response::builder()
//!         .status(StatusCode::OK)
//!         .header(header::CONTENT_TYPE, "application/zip")
//!         .body(Body::wrap_stream(ReaderStream::new(r)))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let address = ([127, 0, 0, 1], 8081).into();
//!     let service =
//!         make_service_fn(|_| async { Ok::<_, hyper::http::Error>(service_fn(zip_archive)) });
//!     let server = Server::bind(&address).serve(service);
//!
//!     println!("Listening on http://{}", address);
//!     server.await?;
//!
//!     Ok(())
//! }
//!
//! ```
//!
//!
//!
//! ## Disclaimer
//! The library is in construction and not stable. More tests need to be done.
//!
//!This implementation is inspired by :
//!- <https://github.com/scotow/zipit> and
//!- <https://github.com/zip-rs/zip>

mod constants;

mod archive_common;
pub mod compress;
pub mod compression;
pub mod error;
pub mod types;
#[cfg(feature = "experimental")]
pub mod uncompress;
