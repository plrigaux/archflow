use crate::async_write_wrapper::AsyncWriteWrapper;
use async_compression::tokio::write::BzEncoder;
use async_compression::tokio::write::XzEncoder;
use async_compression::tokio::write::ZlibEncoder;
use async_compression::tokio::write::ZstdEncoder;
use crc32fast::Hasher;
use flate2::write::ZlibEncoder as ZlibEncoderFlate;
use flate2::Compression;
use std::io::Error as IoError;
use std::io::Write;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

pub const STORE: u16 = 0;
pub const BZIP2: u16 = 12;
pub const DEFALTE: u16 = 8;
pub const ZSTD: u16 = 93;
pub const XZ: u16 = 95;

#[derive(Debug)]
pub enum Compressor {
    Store(),
    Deflated(),
    DeflatedFate2(),
    BZip2(),
    Zstd(),
    Xz(),
}

impl Compressor {
    pub fn compression_method(&self) -> u16 {
        match self {
            Compressor::Store() => STORE,
            Compressor::Deflated() => DEFALTE,
            Compressor::BZip2() => BZIP2,
            Compressor::DeflatedFate2() => DEFALTE,
            Compressor::Zstd() => ZSTD,
            Compressor::Xz() => XZ,
        }
    }

    pub fn version_needed(&self) -> u16 {
        // higher versions matched first
        match self {
            Compressor::BZip2() => 46,
            _ => 20,
        }
    }

    pub async fn compress<'a, R, W>(
        &self,
        writter: &'a mut AsyncWriteWrapper<W>,
        reader: &'a mut R,
        hasher: &'a mut Hasher,
    ) -> Result<usize, IoError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        match self {
            Compressor::Store() => {
                let mut buf = vec![0; 4096];
                let mut total_read = 0;

                loop {
                    let read = reader.read(&mut buf).await?;
                    if read == 0 {
                        break;
                    }

                    total_read += read;
                    hasher.update(&buf[..read]);
                    writter.write_all(&buf[..read]).await?;
                    //self.sink.write_all(&buf[..read]).await?; // Payload chunk.
                }
                //w.flush().await?;
                //w.shutdown().await?;

                Ok(total_read)
            }
            Compressor::Deflated() => {
                let mut zencoder = ZlibEncoder::new(writter);

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

            Compressor::BZip2() => {
                let mut zencoder = BzEncoder::new(writter);

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

            Compressor::DeflatedFate2() => {
                //TODO chage vec to stream
                let mut zencoder = ZlibEncoderFlate::new(Vec::new(), Compression::default());

                let mut buf = vec![0; 4096];
                let mut total_read = 0;

                loop {
                    let read = reader.read(&mut buf).await?;
                    if read == 0 {
                        break;
                    }

                    total_read += read;
                    hasher.update(&buf[..read]);
                    zencoder.write_all(&buf[..read])?;
                    //self.sink.write_all(&buf[..read]).await?; // Payload chunk.
                }
                let hello = zencoder.finish()?;

                writter.write_all(&hello).await?;

                Ok(total_read)
            }

            Compressor::Zstd() => {
                let mut zencoder = ZstdEncoder::new(writter);

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
            Compressor::Xz() => {
                let mut zencoder = XzEncoder::new(writter);

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
        }
    }
}
