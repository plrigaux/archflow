use std::fmt::Display;

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
    BZip2(),
    Lzma(),
    Zstd(),
    Xz(),
    Unknown(u16),
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
