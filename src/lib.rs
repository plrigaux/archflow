//! ## Features
//!
//! - Stream on the fly an archive from multiple AsyncRead objects.
//! - Single read / seek free implementation (the CRC and file size are calculated while streaming and are sent afterwards).
//! - Archive size pre-calculation (useful if you want to set the `Content-Length` before streaming).
//! - [futures](https://docs.rs/futures/latest/futures/) and [tokio](https://docs.rs/tokio/latest/tokio/io/index.html) `AsyncRead` / `AsyncWrite` compatible. Enable either the `futures-async-io` or the `tokio-async-io` feature accordingly.
//!
//! ## Limitations
//!
//! - No compression (stored method only).
//! - Only files (no directories).
//! - No customizable external file attributes.
//!
//! ## Examples
//!
//! ### [File system](examples/fs.rs)
//!
//! Write a zip archive to the file system using [`tokio::fs::File`](https://docs.rs/tokio/1.13.0/tokio/fs/struct.File.html):
//!
//! ```rust
//! use std::io::Cursor;
//! use tokio::fs::File;
//! use zipstream::{Archive, FileDateTime};
//!
//! #[tokio::main]
//! async fn main() {
//!     let file = File::create("archive.zip").await.unwrap();
//!     let mut archive = Archive::new(file);
//!     archive.append(
//!         "file1.txt".to_owned(),
//!         FileDateTime::now(),
//!         &mut Cursor::new(b"hello\n".to_vec()),
//!     ).await.unwrap();
//!     archive.append(
//!         "file2.txt".to_owned(),
//!         FileDateTime::now(),
//!         &mut Cursor::new(b"world\n".to_vec()),
//!     ).await.unwrap();
//!     archive.finalize().await.unwrap();
//! }
//! ```
//!
//! ### [Hyper](examples/hyper.rs)
//!
//! Stream a zip archive as a [`hyper`](https://docs.rs/hyper/0.14.14/hyper/) response:
//!
//! ```rust
//! use std::io::Cursor;
//! use hyper::{header, Body, Request, Response, Server, StatusCode};
//! use tokio::io::duplex;
//! use tokio_util::io::ReaderStream;
//! use zipstream::{archive_size, Archive, FileDateTime};
//!
//! async fn zip_archive(_req: Request<Body>) -> Result<Response<Body>, hyper::http::Error> {
//!     let (filename_1, mut fd_1) = (String::from("file1.txt"), Cursor::new(b"hello\n".to_vec()));
//!     let (filename_2, mut fd_2) = (String::from("file2.txt"), Cursor::new(b"world\n".to_vec()));
//!     let archive_size = archive_size([
//!         (filename_1.as_ref(), fd_1.get_ref().len()),
//!         (filename_2.as_ref(), fd_2.get_ref().len()),
//!     ]);
//!
//!     let (w, r) = duplex(4096);
//!     tokio::spawn(async move {
//!         let mut archive = Archive::new(w);
//!         archive
//!             .append(
//!                 filename_1,
//!                 FileDateTime::now(),
//!                 &mut fd_1,
//!             )
//!             .await
//!             .unwrap();
//!         archive
//!             .append(
//!                 filename_2,
//!                 FileDateTime::now(),
//!                 &mut fd_2,
//!             )
//!             .await
//!             .unwrap();
//!         archive.finalize().await.unwrap();
//!     });
//!
//!     Response::builder()
//!         .status(StatusCode::OK)
//!         .header(header::CONTENT_LENGTH, archive_size)
//!         .header(header::CONTENT_TYPE, "application/zip")
//!         .body(Body::wrap_stream(ReaderStream::new(r)))
//! }
//! ```
use std::future::Future;
use std::mem::size_of;
use std::{io::Error as IoError, pin::Pin};

use async_compression::tokio::write::ZlibEncoder;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

use crc32fast::Hasher;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
struct FileInfo {
    name: String,
    size: usize,
    crc: u32,
    offset: usize,
    datetime: (u16, u16),
}

/// The (timezone-less) date and time that will be written in the archive alongside the file.
///
/// Use `FileDateTime::Zero` if the date and time are insignificant. This will set the value to 0 which is 1980, January 1th, 12AM.  
/// Use `FileDateTime::Custom` if you need to set a custom date and time.  
/// Use `FileDateTime::now()` if you want to use the current date and time (`chrono-datetime` feature required).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FileDateTime {
    /// 1980, January 1th, 12AM.
    Zero,
    /// (year, month, day, hour, minute, second)
    Custom {
        year: u16,
        month: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
    },
}

impl FileDateTime {
    fn tuple(&self) -> (u16, u16, u16, u16, u16, u16) {
        match self {
            FileDateTime::Zero => Default::default(),
            &FileDateTime::Custom {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => (year, month, day, hour, minute, second),
        }
    }

    fn ms_dos(&self) -> (u16, u16) {
        let (year, month, day, hour, min, sec) = self.tuple();
        (
            day | month << 5 | year.saturating_sub(1980) << 9,
            (sec / 2) | min << 5 | hour << 11,
        )
    }

    /// Use the local date and time of the system.
    pub fn now() -> Self {
        Self::from_chrono_datetime(Local::now())
    }

    /// Use a custom date and time.
    pub fn from_chrono_datetime<Tz: TimeZone>(datetime: DateTime<Tz>) -> Self {
        Self::Custom {
            year: datetime.year() as u16,
            month: datetime.month() as u16,
            day: datetime.day() as u16,
            hour: datetime.hour() as u16,
            minute: datetime.minute() as u16,
            second: datetime.second() as u16,
        }
    }
}

macro_rules! header {
    [$capacity:expr; $($elem:expr),*$(,)?] => {
        {
            let mut header = Vec::with_capacity($capacity);
            $(
                header.extend_from_slice(&$elem.to_le_bytes());
            )*
            header
        }
    };
}

const FILE_HEADER_BASE_SIZE: usize = 7 * size_of::<u16>() + 4 * size_of::<u32>();
const DESCRIPTOR_SIZE: usize = 4 * size_of::<u32>();
const CENTRAL_DIRECTORY_ENTRY_BASE_SIZE: usize = 11 * size_of::<u16>() + 6 * size_of::<u32>();
const END_OF_CENTRAL_DIRECTORY_SIZE: usize = 5 * size_of::<u16>() + 3 * size_of::<u32>();

/// A streamed zip archive.
///
/// Create an archive using the `new` function and a `AsyncWrite`. Then, append files one by one using the `append` function. When finished, use the `finalize` function.
///
/// ## Example
///
/// ```no_run
/// use std::io::Cursor;
/// use zipstream::{Archive, FileDateTime};
///
/// #[tokio::main]
/// async fn main() {
///     let mut archive = Archive::new(Vec::new());
///     archive.append(
///         "file1.txt".to_owned(),
///         FileDateTime::now(),
///         &mut Cursor::new(b"hello\n".to_vec()),
///     ).await.unwrap();
///     archive.append(
///         "file2.txt".to_owned(),
///         FileDateTime::now(),
///         &mut Cursor::new(b"world\n".to_vec()),
///     ).await.unwrap();
///     let data = archive.finalize().await.unwrap();
///     println!("{:?}", data);
/// }
/// ```
#[derive(Debug)]
pub struct Archive<W: tokio::io::AsyncWrite + Unpin> {
    sink: AsyncWriteWrapper<W>,
    files_info: Vec<FileInfo>,
    pub written: usize,
}

impl<W: tokio::io::AsyncWrite + Unpin> Archive<W> {
    /// Create a new zip archive, using the underlying `AsyncWrite` to write files' header and payload.
    pub fn new(sink_: W) -> Self {
        //let buf = BufWriter::new(sink_);
        Self {
            sink: AsyncWriteWrapper::new(sink_),
            files_info: Vec::new(),
            written: 0,
        }
    }

    pub fn retrieve_writer(self) -> W {
        self.sink.writer
    }

    pub fn get_archive_size(&self) -> usize {
        self.sink.compress_length
    }

    /// Append a new file to the archive using the provided name, date/time and `AsyncRead` object.  
    /// Filename must be valid UTF-8. Some (very) old zip utilities might mess up filenames during extraction if they contain non-ascii characters.  
    /// File's payload is not compressed and is given `rw-r--r--` permissions.
    ///
    /// # Error
    ///
    /// This function will forward any error found while trying to read from the file stream or while writing to the underlying sink.
    ///
    /// # Features
    ///
    /// Requires `tokio-async-io` feature. `futures-async-io` is also available.
    pub async fn append<R>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        self.append_base(name, datetime, reader, 0, Self::compress_store)
            .await?;

        Ok(())
    }

    async fn compress_store<A, R>(
        w: &mut AsyncWriteWrapper<A>,
        reader: &mut R,
        hasher: &mut Hasher,
    ) -> Result<usize, IoError>
    where
        A: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        //let mut zencoder = ZlibEncoder::with_quality(w, async_compression::Level::Best);

        let mut buf = vec![0; 4096];
        let mut total_read = 0;

        loop {
            let read = reader.read(&mut buf).await?;
            if read == 0 {
                break;
            }

            total_read += read;
            hasher.update(&buf[..read]);
            w.write_all(&buf[..read]).await?;
            //self.sink.write_all(&buf[..read]).await?; // Payload chunk.
        }
        w.shutdown().await?;

        Ok(total_read)
    }

    pub async fn appendzip<R>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        self.append_base(name, datetime, reader, 8, Self::compress_zip)
            .await?;
        Ok(())
    }

    async fn append_base<R, F>(
        &mut self,
        name: String,
        datetime: FileDateTime,
        reader: &mut R,
        compression_method: u16,
        compressor: F,
    ) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
        F: for<'a> AsyncFn<
            &'a mut AsyncWriteWrapper<W>,
            &'a mut R,
            &'a mut Hasher,
            Output = Result<usize, IoError>,
        >,
    {
        let (date, time) = datetime.ms_dos();
        let offset = self.written;
        let mut header = header![
            FILE_HEADER_BASE_SIZE + name.len();
            0x04034b50u32,          // Local file header signature.
            10u16,                  // Version needed to extract.
            1u16 << 3 | 1 << 11,    // General purpose flag (temporary crc and sizes + UTF-8 filename).
            compression_method,     // Compression method .
            time,                   // Modification time.
            date,                   // Modification date.
            0u32,                   // Temporary CRC32.
            0u32,                   // Temporary compressed size.
            0u32,                   // Temporary uncompressed size.
            name.len() as u16,      // Filename length.
            0u16,                   // Extra field length.
        ];
        header.extend_from_slice(name.as_bytes()); // Filename.
        self.sink.write_all(&header).await?;
        self.written += header.len();
        let mut hasher = Hasher::new();
        let cur_size = self.sink.compress_length;

        let total_read = compressor(&mut self.sink, reader, &mut hasher).await?;

        let total_compress = self.sink.compress_length - cur_size;
        println!("total_read {:?}", total_read);
        println!("total_compress {:?}", total_compress);

        let crc = hasher.finalize();
        self.written += total_read;

        let descriptor = header![
            DESCRIPTOR_SIZE;
            0x08074b50u32,      // Data descriptor signature.
            crc,                // CRC32.
            total_compress as u32,  // Compressed size.
            total_read as u32,  // Uncompressed size.
        ];
        self.sink.write_all(&descriptor).await?;
        self.written += descriptor.len();

        self.files_info.push(FileInfo {
            name,
            size: total_read,
            crc,
            offset,
            datetime: (date, time),
        });

        Ok(())
    }

    async fn compress_zip<A, R>(
        w: &mut AsyncWriteWrapper<A>,
        reader: &mut R,
        hasher: &mut Hasher,
    ) -> Result<usize, IoError>
    where
        A: AsyncWrite + Unpin,
        R: AsyncRead + Unpin,
    {
        let mut zencoder = ZlibEncoder::with_quality(w, async_compression::Level::Best);

        let mut buf = vec![0; 4096];
        let mut total_read = 0;

        loop {
            let read = reader.read(&mut buf).await?;
            if read == 0 {
                break;
            }

            total_read += read;
            hasher.update(&buf[..read]);
            zencoder.write_all(&buf[..read]).await?;
            //self.sink.write_all(&buf[..read]).await?; // Payload chunk.
        }
        zencoder.shutdown().await?;

        Ok(total_read)
    }

    /// Finalize the archive by writing the necessary metadata to the end of the archive.
    ///
    /// # Error
    ///
    /// This function will forward any error found while trying to read from the file stream or while writing to the underlying sink.
    ///
    /// # Features
    ///
    /// Requires `tokio-async-io` feature. `futures-async-io` is also available.
    pub async fn finalize(&mut self) -> Result<(), IoError>
    where
        W: AsyncWrite + Unpin,
    {
        let mut central_directory_size = 0;
        for file_info in &self.files_info {
            let mut entry = header![
                CENTRAL_DIRECTORY_ENTRY_BASE_SIZE + file_info.name.len();
                0x02014b50u32,                  // Central directory entry signature.
                0x031eu16,                      // Version made by.
                10u16,                          // Version needed to extract.
                1u16 << 3 | 1 << 11,            // General purpose flag (temporary crc and sizes + UTF-8 filename).
                0u16,                           // Compression method (store).
                file_info.datetime.1,           // Modification time.
                file_info.datetime.0,           // Modification date.
                file_info.crc,                  // CRC32.
                file_info.size as u32,          // Compressed size.
                file_info.size as u32,          // Uncompressed size.
                file_info.name.len() as u16,    // Filename length.
                0u16,                           // Extra field length.
                0u16,                           // File comment length.
                0u16,                           // File's Disk number.
                0u16,                           // Internal file attributes.
                (0o100000u32 | 0o0000400 | 0o0000200 | 0o0000040 | 0o0000004) << 16, // External file attributes (regular file / rw-r--r--).
                file_info.offset as u32,        // Offset from start of file to local file header.
            ];
            entry.extend_from_slice(file_info.name.as_bytes()); // Filename.
            self.sink.write_all(&entry).await?;
            central_directory_size += entry.len();
        }

        let end_of_central_directory = header![
            END_OF_CENTRAL_DIRECTORY_SIZE;
            0x06054b50u32,                  // End of central directory signature.
            0u16,                           // Number of this disk.
            0u16,                           // Number of the disk where central directory starts.
            self.files_info.len() as u16,   // Number of central directory records on this disk.
            self.files_info.len() as u16,   // Total number of central directory records.
            central_directory_size as u32,  // Size of central directory.
            self.written as u32,            // Offset from start of file to central directory.
            0u16,                           // Comment length.
        ];
        self.sink.write_all(&end_of_central_directory).await?;

        Ok(())
    }
}

/// Calculate the size that an archive could be based on the names and sizes of files.
///
/// ## Example
///
/// ```no_run
///
/// use zipstream::archive_size;
///
/// assert_eq!(
///     archive_size([
///         ("file1.txt", b"hello\n".len()),
///         ("file2.txt", b"world\n".len()),
///     ]),
///     254,
/// );
/// ```
pub fn archive_size<'a, I: IntoIterator<Item = (&'a str, usize)>>(files: I) -> usize {
    files
        .into_iter()
        .map(|(name, size)| {
            FILE_HEADER_BASE_SIZE
                + name.len()
                + size
                + DESCRIPTOR_SIZE
                + CENTRAL_DIRECTORY_ENTRY_BASE_SIZE
                + name.len()
        })
        .sum::<usize>()
        + END_OF_CENTRAL_DIRECTORY_SIZE
}

#[derive(Debug)]
struct AsyncWriteWrapper<W: AsyncWrite + Unpin> {
    writer: W,
    compress_length: usize,
}

impl<W: AsyncWrite + Unpin> AsyncWriteWrapper<W> {
    fn new(w: W) -> AsyncWriteWrapper<W> {
        Self {
            writer: w,
            compress_length: 0,
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for AsyncWriteWrapper<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let wrapper = self.get_mut();
        wrapper.compress_length += buf.len();
        Pin::new(&mut wrapper.writer).poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_shutdown(cx)
    }
}

trait AsyncFn<T, U, V>: Fn(T, U, V) -> <Self as AsyncFn<T, U, V>>::Fut {
    type Fut: Future<Output = <Self as AsyncFn<T, U, V>>::Output>;
    type Output;
}

impl<T, U, V, F, Fut> AsyncFn<T, U, V> for F
where
    F: Fn(T, U, V) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
}
