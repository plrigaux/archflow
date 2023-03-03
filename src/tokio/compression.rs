use async_compression::tokio::write::BzEncoder;
use async_compression::tokio::write::DeflateEncoder;
use async_compression::tokio::write::LzmaEncoder;
use async_compression::tokio::write::XzEncoder;

use async_compression::tokio::write::ZstdEncoder;
use crc32fast::Hasher;

use std::fmt::Display;
use std::io::Error as IoError;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

pub const STORE: u16 = 0;
pub const DEFALTE: u16 = 8;
pub const BZIP2: u16 = 12;
pub const LZMA: u16 = 14;
pub const ZSTD: u16 = 93;
pub const XZ: u16 = 95;

#[derive(Debug, Clone, Copy)]
pub enum Compressor {
    Store(),
    Deflate(),
    //DeflateFate2(),
    BZip2(),
    Lzma(),
    Zstd(),
    Xz(),
    Unknown(u16),
}

macro_rules! compress_tokio {
    ( $encoder:expr, $hasher:expr, $reader:expr) => {{
        let mut buf = vec![0; 4096];
        let mut total_read = 0;

        loop {
            let read = $reader.read(&mut buf).await?;
            if read == 0 {
                break;
            }

            total_read += read;
            $hasher.update(&buf[..read]);
            $encoder.write_all(&buf[..read]).await?;
            //self.sink.write_all(&buf[..read]).await?; // Payload chunk.
        }
        $encoder.flush().await?;
        $encoder.shutdown().await?;

        total_read
    }};
}

impl Compressor {
    pub fn compression_method(&self) -> u16 {
        match self {
            Compressor::Store() => STORE,
            Compressor::Deflate() => DEFALTE,
            //   Compressor::DeflateFate2() => DEFALTE,
            Compressor::BZip2() => BZIP2,
            Compressor::Lzma() => LZMA,
            Compressor::Zstd() => ZSTD,
            Compressor::Xz() => XZ,
            Compressor::Unknown(compression_method) => *compression_method,
        }
    }

    pub fn version_needed(&self) -> u16 {
        // higher versions matched first
        match self {
            Compressor::Lzma() => 63,
            Compressor::Zstd() => 63,
            Compressor::BZip2() => 46,
            _ => 20,
        }
    }

    pub fn from_compression_method(compression_method: u16) -> Compressor {
        // higher versions matched first
        match compression_method {
            STORE => Compressor::Store(),
            DEFALTE => Compressor::Deflate(),
            BZIP2 => Compressor::BZip2(),
            LZMA => Compressor::Lzma(),
            ZSTD => Compressor::Zstd(),
            XZ => Compressor::Xz(),
            _ => Compressor::Unknown(compression_method),
        }
    }

    pub fn compression_method_label(&self) -> &str {
        // higher versions matched first
        match self {
            Compressor::Store() => "store",
            Compressor::Deflate() => "deflate",
            //Compressor::DeflateFate2() => "deflate",
            Compressor::BZip2() => "bzip2",
            Compressor::Lzma() => "lzma",
            Compressor::Zstd() => "zstd",
            Compressor::Xz() => "xz",
            Compressor::Unknown(_) => "unknown",
        }
    }

    pub async fn compress<'a, R, W>(
        &self,
        writer: &'a mut W,
        reader: &'a mut R,
        hasher: &'a mut Hasher,
        compression_level: Level,
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
                    writer.write_all(&buf[..read]).await?;
                }
                writer.flush().await?;

                Ok(total_read)
            }
            Compressor::Deflate() => {
                let mut zencoder = DeflateEncoder::with_quality(writer, compression_level.value());

                let total_read = compress_tokio!(zencoder, hasher, reader);

                Ok(total_read)
            }

            Compressor::BZip2() => {
                let mut zencoder = BzEncoder::with_quality(writer, compression_level.value());

                let total_read = compress_tokio!(zencoder, hasher, reader);

                Ok(total_read)
            }
            Compressor::Lzma() => {
                let mut zencoder = LzmaEncoder::with_quality(writer, compression_level.value());

                let total_read = compress_tokio!(zencoder, hasher, reader);

                Ok(total_read)
            }
            Compressor::Zstd() => {
                let mut zencoder = ZstdEncoder::with_quality(writer, compression_level.value());

                let total_read = compress_tokio!(zencoder, hasher, reader);

                Ok(total_read)
            }
            Compressor::Xz() => {
                let mut zencoder = XzEncoder::with_quality(writer, compression_level.value());

                let total_read = compress_tokio!(zencoder, hasher, reader);

                Ok(total_read)
            }
            Compressor::Unknown(compression_method) => {
                panic!("unsupported compression method {:?}", compression_method)
            }
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Compressor::Unknown(_))
    }
}

impl Display for Compressor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.compression_method_label())
    }
}

#[derive(Clone, Copy)]
pub enum Level {
    Fastest,
    Best,
    Default,
    Precise(u32),
}

impl Level {
    fn value(&self) -> async_compression::Level {
        match self {
            Level::Fastest => async_compression::Level::Fastest,
            Level::Best => async_compression::Level::Best,
            Level::Default => async_compression::Level::Default,
            Level::Precise(val) => async_compression::Level::Precise(*val),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tokio::async_wrapper::AsyncWriteWrapper;

    use super::*;
    use async_compression::tokio::write::ZlibEncoder;
    use flate2::write::DeflateEncoder as DeflateEncoderFlate2;
    use flate2::write::ZlibEncoder as ZlibEncoderFlate;
    use std::io::Write;
    #[tokio::test]
    async fn test_defate_basic() {
        let x = b"example";
        let mut e = ZlibEncoder::new(Vec::new());
        e.write_all(x).await.unwrap();
        e.flush().await.unwrap();
        e.shutdown().await.unwrap();
        let temp = e.into_inner();
        println!("compress len {:?}", temp.len());
        println!("{:02X?}", temp);

        // [0x74, 78 9C 4A AD 48 CC 2D C8 49 05 00 00 00 FF FF 03 00 0B C0 02 ED]

        // [120, 156, 74, 173, 72, 204, 45, 200, 73, 5, 0, 0, 0, 255, 255]
        // import zlib; print(zlib.decompress(bytes([120, 156, 74, 173, 72, 204, 45, 200, 73, 5, 0, 0, 0, 255, 255])))
        // fail with:
        // Traceback (most recent call last):
        //   File "test.py", line 25, in <module>
        //     print(zlib.decompress(bytes([120, 156, 74, 173, 72, 204, 45, 200, 73, 5, 0, 0, 0, 255, 255])))
        // zlib.error: Error -5 while decompressing data: incomplete or truncated stream

        // Working code with flate2
        let mut encoder = ZlibEncoderFlate::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(x).unwrap();
        encoder.flush().unwrap();
        let temp = encoder.finish().unwrap();
        println!("compress len {:?}", temp.len());
        println!("{:02X?}", temp);
        // [120, 1, 0, 7, 0, 248, 255, 101, 120, 97, 109, 112, 108, 101, 0, 0, 0, 255, 255, 3, 0, 11, 192, 2, 237]
        // import zlib; print(zlib.decompress(bytes([120, 1, 0, 7, 0, 248, 255, 101, 120, 97, 109, 112, 108, 101, 0, 0, 0, 255, 255, 3, 0, 11, 192, 2, 237])))
        // prints b'example`

        let mut encoder = DeflateEncoder::new(Vec::new());
        encoder.write_all(x).await.unwrap();
        encoder.flush().await.unwrap();
        encoder.shutdown().await.unwrap();
        let temp = encoder.into_inner();
        println!("compress len {:?}", temp.len());
        println!("{:02X?}", temp);

        let mut encoder = DeflateEncoderFlate2::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(x).unwrap();
        encoder.flush().unwrap();
        let temp = encoder.finish().unwrap();
        println!("compress len {:?}", temp.len());
        println!("{:02X?}", temp);
    }

    #[tokio::test]
    async fn test_defate_compressor() {
        let x = b"example";

        let compressor = Compressor::Deflate();
        let mut hasher = Hasher::new();

        //let a: AsyncRead = &x;
        let mut writer = AsyncWriteWrapper::new(Vec::new());
        compressor
            .compress(&mut writer, &mut x.as_ref(), &mut hasher, Level::Default)
            .await
            .unwrap();

        let temp = writer.retrieve_writer();
        println!("compress len {:?}", temp.len());
        println!("{:X?}", temp);
    }
}

//74 78 9C 4A AD 48 CC 2D C8 49 05 00 00 00 FF FF 03 00 0B C0 02 ED
