use std::io::{Read, Write};

use bzip2::write::BzEncoder;
use crc32fast::Hasher;
use flate2::{write::DeflateEncoder, Compression};

use xz2::write::XzEncoder;

use crate::{
    compress::common::{compress_common, compress_common_std, is_text_buf},
    compression::{CompressionMethod, Level},
    error::ArchiveError,
};

/* macro_rules! compress_common {
    ( $encoder:expr, $hasher:expr, $reader:expr) => {{
        let mut buf = vec![0; 4096];
        let mut total_read: u64 = 0;

        let mut read = $reader.read(&mut buf)?;
        let is_text = is_text_buf(&buf[..read]);
        while read != 0 {
            total_read += read as u64;
            $hasher.update(&buf[..read]);
            $encoder.write_all(&buf[..read])?;
            read = $reader.read(&mut buf)?;
        }
        $encoder.finish()?;

        (total_read, is_text)
    }};
}
 */
impl From<Level> for flate2::Compression {
    fn from(level: Level) -> Self {
        match level {
            Level::Fastest => Compression::fast(),
            Level::Best => Compression::best(),
            Level::Default => Compression::default(),
            Level::Precise(val) => Compression::new(val as u32),
            Level::None => Compression::none(),
        }
    }
}

impl From<Level> for bzip2::Compression {
    fn from(level: Level) -> Self {
        match level {
            Level::Fastest => bzip2::Compression::fast(),
            Level::Best => bzip2::Compression::best(),
            Level::Default => bzip2::Compression::default(),
            Level::Precise(val) => bzip2::Compression::new(val as u32),
            Level::None => bzip2::Compression::none(),
        }
    }
}

impl From<Level> for u32 {
    fn from(level: Level) -> Self {
        match level {
            Level::Fastest => 1,
            Level::Best => 9,
            Level::Default => 6,
            Level::Precise(val) => val as u32,
            Level::None => 0,
        }
    }
}

pub fn compress<'a, R, W>(
    compressor: CompressionMethod,
    writer: &'a mut W,
    reader: &'a mut R,
    hasher: &'a mut Hasher,
    compression_level: Level,
) -> Result<(u64, bool), ArchiveError>
where
    R: Read,
    W: Write + ?Sized,
{
    match compressor {
        CompressionMethod::Store() => {
            let mut buf = vec![0; 4096];
            let mut total_read: u64 = 0;

            let mut read = reader.read(&mut buf)?;
            let is_text = is_text_buf(&buf[..read]);

            while read != 0 {
                total_read += read as u64;
                hasher.update(&buf[..read]);
                writer.write_all(&buf[..read])?;

                read = reader.read(&mut buf)?;
            }
            writer.flush()?;

            Ok((total_read, is_text))
        }

        CompressionMethod::Deflate() => {
            let mut encoder = DeflateEncoder::new(writer, compression_level.into());

            let total_read = compress_common_std!(encoder, hasher, reader);

            Ok(total_read)
        }

        CompressionMethod::BZip2() => {
            let mut encoder = BzEncoder::new(writer, compression_level.into());

            let total_read = compress_common_std!(encoder, hasher, reader);

            Ok(total_read)
        }

        CompressionMethod::Zstd() => {
            let zstd_compression_level = match compression_level {
                Level::Fastest => Ok(1),
                Level::Best => Ok(22),
                Level::Default => Ok(zstd::DEFAULT_COMPRESSION_LEVEL),
                Level::None => Err(ArchiveError::UnsuportedCompressionLevel(compressor)),
                Level::Precise(val) => Ok(val),
            }?;

            let mut encoder = zstd::stream::write::Encoder::new(writer, zstd_compression_level)?;
            let total_read = compress_common_std!(encoder, hasher, reader);

            Ok(total_read)
        }
        CompressionMethod::Xz() => {
            let mut encoder = XzEncoder::new(writer, compression_level.into());

            let total_read = compress_common_std!(encoder, hasher, reader);

            Ok(total_read)
        }

        _ => Err(ArchiveError::UnsuportedCompressionMethod(compressor)),
    }
}

#[cfg(test)]
mod test {
    use crate::compress::std::write_wrapper::{CommonWrapper, WriteWrapper};
    use crate::compression::Level;

    use super::*;

    use flate2::write::ZlibEncoder as ZlibEncoderFlate;
    use std::io::Write;
    #[test]
    fn test_defate_basic() {
        let x = b"example";

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
    }

    #[test]
    fn test_defate_compressor() {
        let x = b"example";

        let compressor = CompressionMethod::Deflate();
        let mut hasher = Hasher::new();

        //let a: AsyncRead = &x;
        let mut writer: Box<dyn CommonWrapper<Vec<u8>>> = Box::new(WriteWrapper::new(Vec::new()));

        compress(
            compressor,
            &mut writer,
            &mut x.as_ref(),
            &mut hasher,
            Level::Default,
        )
        .unwrap();

        let temp = writer.get_into();
        println!("compress len {:?}", temp.len());
        println!("{:X?}", temp);
    }

    #[test]
    fn test_zstd_level() {
        let range = zstd::compression_level_range();
        println!("range: {:?}", range);

        let default = zstd::DEFAULT_COMPRESSION_LEVEL;

        println!("range min : {:?}", default);
    }
}

//74 78 9C 4A AD 48 CC 2D C8 49 05 00 00 00 FF FF 03 00 0B C0 02 ED
