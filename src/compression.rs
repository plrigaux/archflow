use std::fmt::Display;

use crate::error::ArchiveError;

pub const STORE: u16 = 0;
pub const DEFALTE: u16 = 8;
pub const BZIP2: u16 = 12;
pub const LZMA: u16 = 14;
pub const ZSTD: u16 = 93;
pub const XZ: u16 = 95;

#[derive(Debug, Clone, Copy)]
pub enum CompressionMethod {
    Store(),
    Deflate(),
    BZip2(),
    Lzma(),
    Zstd(),
    Xz(),
    Unknown(u16),
}

impl CompressionMethod {
    pub fn zip_code(&self) -> u16 {
        match self {
            CompressionMethod::Store() => STORE,
            CompressionMethod::Deflate() => DEFALTE,
            CompressionMethod::BZip2() => BZIP2,
            CompressionMethod::Lzma() => LZMA,
            CompressionMethod::Zstd() => ZSTD,
            CompressionMethod::Xz() => XZ,
            CompressionMethod::Unknown(comp_method_code) => *comp_method_code,
        }
    }

    pub fn zip_version_needed(&self) -> u16 {
        // higher versions matched first
        match self {
            CompressionMethod::Lzma() => 63,
            CompressionMethod::Zstd() => 63,
            CompressionMethod::BZip2() => 46,
            _ => 20,
        }
    }

    pub fn from_compression_method(
        compression_method: u16,
    ) -> Result<CompressionMethod, ArchiveError> {
        // higher versions matched first
        match compression_method {
            STORE => Ok(CompressionMethod::Store()),
            DEFALTE => Ok(CompressionMethod::Deflate()),
            BZIP2 => Ok(CompressionMethod::BZip2()),
            LZMA => Ok(CompressionMethod::Lzma()),
            ZSTD => Ok(CompressionMethod::Zstd()),
            XZ => Ok(CompressionMethod::Xz()),
            _ => Err(ArchiveError::UnsuportedCompressionMethodCode(
                compression_method,
            )),
        }
    }

    pub fn label(&self) -> &str {
        // higher versions matched first
        match self {
            CompressionMethod::Store() => "store",
            CompressionMethod::Deflate() => "deflate",
            CompressionMethod::BZip2() => "bzip2",
            CompressionMethod::Lzma() => "lzma",
            CompressionMethod::Zstd() => "zstd",
            CompressionMethod::Xz() => "xz",
            CompressionMethod::Unknown(_) => "unknown",
        }
    }

    pub fn update_general_purpose_bit_flag(&self, flag: u16, level: Level) -> u16 {
        const BIT1: u16 = 1 << 1; //2
        const BIT2: u16 = 1 << 2; //4
        match self {
            CompressionMethod::Deflate() => match level {
                Level::Fastest => flag | BIT2, //1      1    Super Fast (-es) compression option was used. //1      0    Fast (-ef) compression option was used.
                Level::Best => flag | BIT1, //0      1    Maximum (-exx/-ex) compression option was used.
                Level::Default => flag,     //0      0    Normal (-en) compression option was used.
                Level::Precise(val) => match val {
                    1..=2 => self.update_general_purpose_bit_flag(flag, Level::Fastest),
                    6 => self.update_general_purpose_bit_flag(flag, Level::Default),
                    8.. => self.update_general_purpose_bit_flag(flag, Level::Best),
                    _ => flag,
                },
                Level::None => flag,
            },

            _ => flag,
        }
    }
}

impl Display for CompressionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Clone, Copy)]
pub enum Level {
    Fastest,
    Best,
    Default,
    None,
    Precise(i32),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn update_general_purpose_bit_flag() {
        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Default),
            0
        );

        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(123, Level::Default),
            123
        );

        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Best),
            CompressionMethod::Deflate()
                .update_general_purpose_bit_flag(0, Level::Precise(1252345))
        );

        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Best),
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Precise(8))
        );

        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Fastest),
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Precise(1))
        );

        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Fastest),
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Precise(2))
        );

        assert_eq!(
            CompressionMethod::Deflate().update_general_purpose_bit_flag(0, Level::Fastest),
            4
        );

        assert_eq!(
            CompressionMethod::Store().update_general_purpose_bit_flag(0, Level::Fastest),
            0
        );
    }
}
