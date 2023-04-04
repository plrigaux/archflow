use async_compression::tokio::write::{BzEncoder, DeflateEncoder, XzEncoder, ZstdEncoder};
use crc32fast::Hasher;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    compress::common::{compress_common, compress_common_async, is_text_buf},
    compression::{CompressionMethod, Level},
    error::ArchiveError,
};

/* macro_rules! compress_common {
    ( $encoder:expr, $hasher:expr, $reader:expr) => {{
        let mut buf = vec![0; 4096];
        let mut total_read: u64 = 0;

        let mut read = $reader.read(&mut buf).await?;
        let is_text = is_text_buf(&buf[..read]);

        while read != 0 {
            total_read += read as u64;
            $hasher.update(&buf[..read]);
            $encoder.write_all(&buf[..read]).await?;
            read = $reader.read(&mut buf).await?;
        }
        $encoder.flush().await?;

        $encoder.shutdown().await?;
        (total_read, is_text)
    }};
}
 */
impl From<Level> for async_compression::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Fastest => async_compression::Level::Fastest,
            Level::Best => async_compression::Level::Best,
            Level::Default => async_compression::Level::Default,
            Level::Precise(val) => async_compression::Level::Precise(val as u32),
            Level::None => async_compression::Level::Precise(0),
        }
    }
}

pub async fn compress<'a, R, W>(
    compressor: CompressionMethod,
    writer: &'a mut W,
    reader: &'a mut R,
    hasher: &'a mut Hasher,
    compression_level: Level,
) -> Result<(u64, bool), ArchiveError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let method = if compression_level == Level::None {
        CompressionMethod::Store()
    } else {
        compressor
    };

    match method {
        CompressionMethod::Store() => {
            let mut buf = vec![0; 4096];
            let mut total_read: u64 = 0;

            let mut read = reader.read(&mut buf).await?;
            let is_text = is_text_buf(&buf[..read]);

            while read != 0 {
                total_read += read as u64;
                hasher.update(&buf[..read]);
                writer.write_all(&buf[..read]).await?;

                read = reader.read(&mut buf).await?;
            }
            writer.flush().await?;

            Ok((total_read, is_text))
        }
        CompressionMethod::Deflate() => {
            let mut zencoder = DeflateEncoder::with_quality(writer, compression_level.into());

            let total_read = compress_common_async!(zencoder, hasher, reader);

            Ok(total_read)
        }

        CompressionMethod::BZip2() => {
            let mut encoder = BzEncoder::with_quality(writer, compression_level.into());

            let total_read = compress_common_async!(encoder, hasher, reader);

            Ok(total_read)
        }

        CompressionMethod::Zstd() => {
            let mut encoder = ZstdEncoder::with_quality(writer, compression_level.into());

            let total_read = compress_common_async!(encoder, hasher, reader);

            Ok(total_read)
        }
        CompressionMethod::Xz() => {
            //let bw = BufWriter::new(writer);
            let mut encoder = XzEncoder::with_quality(writer, compression_level.into());

            let total_read = compress_common_async!(encoder, hasher, reader);

            Ok(total_read)
        }
        CompressionMethod::Unknown(compression_method_code) => Err(
            ArchiveError::UnsuportedCompressionMethodCode(compression_method_code),
        ),
    }
}

#[cfg(test)]
mod test {
    use crate::compress::tokio::async_wrapper::AsyncWriteWrapper;
    use crate::compress::tokio::async_wrapper::CommonWrapper;
    use crate::compression::Level;

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

        let compressor = CompressionMethod::Deflate();
        let mut hasher = Hasher::new();

        //let a: AsyncRead = &x;
        let mut writer: Box<dyn CommonWrapper<Vec<u8>>> =
            Box::new(AsyncWriteWrapper::new(Vec::new()));

        compress(
            compressor,
            &mut writer,
            &mut x.as_ref(),
            &mut hasher,
            Level::Default,
        )
        .await
        .unwrap();

        let temp = writer.get_into();
        println!("compress len {:?}", temp.len());
        println!("{:X?}", temp);
    }
}

//74 78 9C 4A AD 48 CC 2D C8 49 05 00 00 00 FF FF 03 00 0B C0 02 ED
