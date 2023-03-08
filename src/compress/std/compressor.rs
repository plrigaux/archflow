use std::io::{Read, Write};

use bzip2::write::BzEncoder;
use crc32fast::Hasher;
use flate2::{write::DeflateEncoder, Compression};
use lzma::LzmaWriter;
use xz2::write::XzEncoder;

use crate::{
    compression::{CompressionMethod, Level},
    error::ArchiveError,
};

macro_rules! compress_flate {
    ( $encoder:expr, $hasher:expr, $reader:expr) => {{
        let mut buf = vec![0; 4096];
        let mut total_read = 0;

        loop {
            let read = $reader.read(&mut buf)?;
            if read == 0 {
                break;
            }

            total_read += read;
            $hasher.update(&buf[..read]);
            $encoder.write_all(&buf[..read])?;
        }
        $encoder.flush()?;

        total_read
    }};
}

impl From<Level> for flate2::Compression {
    fn from(level: Level) -> Self {
        match level {
            Level::Fastest => Compression::fast(),
            Level::Best => Compression::best(),
            Level::Default => Compression::default(),
            Level::Precise(val) => Compression::new(val),
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
            Level::Precise(val) => bzip2::Compression::new(val),
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
            Level::Precise(val) => val,
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
) -> Result<usize, ArchiveError>
where
    R: Read,
    W: Write,
{
    match compressor {
        CompressionMethod::Store() => {
            let mut buf = vec![0; 4096];
            let mut total_read = 0;

            loop {
                let read = reader.read(&mut buf)?;
                if read == 0 {
                    break;
                }

                total_read += read;
                hasher.update(&buf[..read]);
                writer.write_all(&buf[..read])?;
            }
            writer.flush()?;

            Ok(total_read)
        }
        CompressionMethod::Deflate() => {
            let mut encoder = DeflateEncoder::new(writer, compression_level.into());

            let total_read = compress_flate!(encoder, hasher, reader);

            Ok(total_read)
        }

        CompressionMethod::BZip2() => {
            let mut encoder = BzEncoder::new(writer, compression_level.into());

            let total_read = compress_flate!(encoder, hasher, reader);

            Ok(total_read)
        }
        CompressionMethod::Lzma() => {
            match compress_lzma(writer, reader, hasher, compression_level) {
                Ok(total_read) => Ok(total_read),
                Err(e) => Err(ArchiveError::LZMA(e)),
            }
        }
        CompressionMethod::Zstd() => {
            let zstd_compression_level = match compression_level {
                Level::Fastest => Ok(1),
                Level::Best => Ok(22),
                Level::Default => Ok(zstd::DEFAULT_COMPRESSION_LEVEL),
                Level::None => Err(ArchiveError::UnsuportedCompressionLevel(compressor)),
                Level::Precise(val) => Ok(val as i32),
            }?;

            let mut encoder = zstd::stream::write::Encoder::new(writer, zstd_compression_level)?;
            let total_read = compress_flate!(encoder, hasher, reader);
            encoder.finish()?;

            Ok(total_read)
        }
        CompressionMethod::Xz() => {
            let mut zencoder = XzEncoder::new(writer, compression_level.into());

            let total_read = compress_flate!(zencoder, hasher, reader);

            Ok(total_read)
        }

        _ => Err(ArchiveError::UnsuportedCompressionMethod(compressor)),
    }
}

fn compress_lzma<'a, R, W>(
    writer: &'a mut W,
    reader: &'a mut R,
    hasher: &'a mut Hasher,
    compression_level: Level,
) -> Result<usize, lzma::LzmaError>
where
    R: Read,
    W: Write,
{
    let lzma_compression_level: u32 = match compression_level {
        Level::Fastest => 1,
        Level::Best => 9,
        Level::Default => 6,
        Level::None => 0,
        Level::Precise(val) => val,
    };

    let mut encoder = LzmaWriter::new_compressor(writer, lzma_compression_level)?;

    let mut buf = vec![0; 4096];
    let mut total_read = 0;

    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }

        total_read += read;
        hasher.update(&buf[..read]);
        encoder.write_all(&buf[..read])?;
    }
    encoder.flush()?;
    encoder.finish()?;
    Ok(total_read)
}

#[cfg(test)]
mod test {
    use crate::compress::std::write_wrapper::WriteWrapper;
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
        let mut writer = WriteWrapper::new(Vec::new());

        compress(
            compressor,
            &mut writer,
            &mut x.as_ref(),
            &mut hasher,
            Level::Default,
        )
        .unwrap();

        let temp = writer.retrieve_writer();
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
