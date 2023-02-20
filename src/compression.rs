use crate::async_write_wrapper::AsyncWriteWrapper;
use async_compression::tokio::write::BzEncoder;
use async_compression::tokio::write::ZlibEncoder;
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

pub enum Compressor {
    Storer(),
    Deflater(),
    Deflater_Fate2(),
    BZip2(),
}

impl Compressor {
    pub fn compression_method(&self) -> u16 {
        match self {
            Compressor::Storer() => STORE,
            Compressor::Deflater() => DEFALTE,
            Compressor::BZip2() => BZIP2,
            Compressor::Deflater_Fate2() => DEFALTE,
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
            Compressor::Storer() => {
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
            Compressor::Deflater() => {
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

            Compressor::Deflater_Fate2() => {
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
                zencoder.flush()?;

                Ok(total_read)
            }
        }
    }
}
