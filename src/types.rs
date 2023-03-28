extern crate chrono;
use core::fmt;
use std::{u16, u8};

use crate::{archive_common::ExtraFields, compression::CompressionMethod};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};

/// The archive file complete information.
///
/// Most of this information is located in the archive central registry and it's partly duplicated in thier respective file header.
#[derive(Debug)]
pub struct ArchiveFileEntry {
    pub version_made_by: u16,
    pub version_needed: u16,
    pub general_purpose_flags: u16,
    pub compression_method: u16,
    pub last_mod_file_time: u16,
    pub last_mod_file_date: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub file_name_len: u16,
    pub extra_field_length: u16,
    pub file_name_as_bytes: Vec<u8>,
    pub offset: u64,
    pub compressor: CompressionMethod,
    pub file_disk_number: u16,
    pub internal_file_attributes: u16,
    pub external_file_attributes: u32,
    pub file_comment: Option<Vec<u8>>,
    pub extra_fields: Vec<Box<dyn ExtraFields>>,
}

impl ArchiveFileEntry {
    pub fn version_needed(&self) -> u16 {
        // higher versions matched first
        match self.compressor {
            CompressionMethod::Zstd() => 63,
            CompressionMethod::BZip2() => 46,
            _ => 20,
        }
    }

    fn extended_local_header(&self) -> bool {
        self.general_purpose_flags & (1u16 << 3) != 0
    }

    fn is_encrypted(&self) -> bool {
        self.general_purpose_flags & (1u16 << 0) != 0
    }

    ///Retreive the version in a pretty format
    fn pretty_version(zip_version: u16) -> (u16, u16) {
        let major = zip_version / 10;
        let minor = zip_version % 10;

        (major, minor)
    }

    pub(crate) fn file_comment_length(&self) -> u16 {
        match &self.file_comment {
            Some(comment) => comment.len() as u16,
            None => 0,
        }
    }

    fn system_origin(&self) -> String {
        let system_code = self.version_made_by.to_be_bytes()[0];
        FileCompatibilitySystem::from_u8(system_code).to_string()
    }

    pub fn get_file_name(&self) -> String {
        String::from_utf8_lossy(&self.file_name_as_bytes).to_string()
    }

    pub fn is_zip64(&self) -> bool {
        self.uncompressed_size >= u32::MAX as u64
    }
}

impl fmt::Display for ArchiveFileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let padding = 48;

        let file_name = String::from_utf8_lossy(&self.file_name_as_bytes);

        writeln!(f, "{}\n", file_name)?;

        writeln!(
            f,
            "{: <padding$}{}",
            "offset of local header from start of archive:", self.offset
        )?;

        writeln!(f, "{: <padding$}({:016X}h) bytes", "", self.offset)?;

        writeln!(
            f,
            "{: <padding$}{}",
            "file system or operating system of origin:",
            self.system_origin()
        )?;

        let (major, minor) = ArchiveFileEntry::pretty_version(self.version_needed);
        writeln!(
            f,
            "{: <padding$}{}.{}",
            "minimum software version required to extract:", major, minor
        )?;

        writeln!(
            f,
            "{: <padding$}{:#016b}",
            "general purpose bit flag:", self.general_purpose_flags
        )?;

        let label = match CompressionMethod::from_compression_method(self.compression_method) {
            Ok(compressor) => compressor.label().to_owned(),
            Err(_) => {
                let str_val = self.compression_method.to_string();
                let mut val = String::from("unknown (");
                val.push_str(&str_val);
                val.push(')');
                val
            }
        };

        writeln!(f, "{: <padding$}{}", "compression method:", label)?;

        let extended_local_header = if self.is_encrypted() {
            "encrypted"
        } else {
            "not encrypted"
        };

        writeln!(
            f,
            "{: <padding$}{}",
            "file security status:", extended_local_header
        )?;

        let extended_local_header = if self.extended_local_header() {
            "yes"
        } else {
            "no"
        };

        writeln!(
            f,
            "{: <padding$}{}",
            "extended local header:", extended_local_header
        )?;

        let date_time = DateTimeCS::from_msdos(self.last_mod_file_date, self.last_mod_file_time);
        writeln!(
            f,
            "{: <padding$}{}",
            "file last modified on (DOS date/time):", date_time
        )?;

        writeln!(
            f,
            "{: <padding$}{:x}",
            "32-bit CRC value (hex):", self.crc32
        )?;

        writeln!(
            f,
            "{: <padding$}{} bytes",
            "compressed size:", self.compressed_size
        )?;
        writeln!(
            f,
            "{: <padding$}{:} bytes",
            "uncompressed size:", self.uncompressed_size
        )?;

        writeln!(
            f,
            "{: <padding$}{:} characters",
            "length of filename:", self.file_name_len
        )?;

        writeln!(
            f,
            "{: <padding$}{:} bytes",
            "length of extra field:", self.extra_field_length
        )?;
        writeln!(
            f,
            "{: <padding$}{:} characters",
            "length of file comment:",
            self.file_comment_length()
        )?;

        if let Some(comment) = &self.file_comment {
            writeln!(
                f,
                "\n------------------------- file comment begins ----------------------------"
            )?;
            let s = String::from_utf8_lossy(comment);
            writeln!(f, "{}", s)?;

            writeln!(
                f,
                "-------------------------- file comment ends -----------------------------"
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DateTimeCS {
    year: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
}

impl Default for DateTimeCS {
    /// Construct a new FileOptions object
    fn default() -> Self {
        Self {
            year: 1980,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

impl DateTimeCS {
    pub fn from_chrono_datetime<Tz: TimeZone>(datetime: DateTime<Tz>) -> Self {
        Self {
            year: datetime.year() as u16,
            month: datetime.month() as u16,
            day: datetime.day() as u16,
            hour: datetime.hour() as u16,
            minute: datetime.minute() as u16,
            second: datetime.second() as u16,
        }
    }

    pub fn now() -> Self {
        Self::from_chrono_datetime(Local::now())
    }

    pub fn from_timestamp(timestamp: i32) -> Self {
        match Utc.timestamp_opt(timestamp as i64, 0) {
            chrono::LocalResult::None => Self::default(),
            chrono::LocalResult::Single(single) => Self::from_chrono_datetime(single),
            chrono::LocalResult::Ambiguous(single, _) => Self::from_chrono_datetime(single),
        }
    }

    pub fn from_msdos(datepart: u16, timepart: u16) -> Self {
        let seconds = (timepart & 0b0000000000011111) << 1;
        let minutes = (timepart & 0b0000011111100000) >> 5;
        let hours = (timepart & 0b1111100000000000) >> 11;
        let days = datepart & 0b0000000000011111;
        let months = (datepart & 0b0000000111100000) >> 5;
        let years = (datepart & 0b1111111000000000) >> 9;

        Self {
            year: years + 1980,
            month: months,
            day: days,
            hour: hours,
            minute: minutes,
            second: seconds,
        }
    }

    pub fn to_time(&self) -> chrono::NaiveDateTime {
        Self::to_time_dry(
            self.year as i32,
            self.month as u32,
            self.day as u32,
            self.hour as u32,
            self.minute as u32,
            self.second as u32,
        )
    }

    fn to_time_dry(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> chrono::NaiveDateTime {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| {
            let zero = DateTimeCS::default();
            NaiveDate::from_ymd_opt(zero.year as i32, zero.month as u32, zero.day as u32).unwrap()
        });

        date.and_hms_opt(hour, minute, second).unwrap_or_default()
    }

    pub fn ms_dos(&self) -> (u16, u16) {
        let date = self.day | (self.month << 5) | self.year.saturating_sub(1980) << 9;
        let time = (self.second / 2) | (self.minute << 5) | self.hour << 11;
        (date, time)
    }

    pub fn to_timestamp(&self) -> i32 {
        let local = &self.to_time();

        match local.and_local_timezone(Utc) {
            chrono::LocalResult::None => Self::default().to_timestamp(),
            chrono::LocalResult::Single(single) => Self::convert_timestamp(single),
            chrono::LocalResult::Ambiguous(first, _) => Self::convert_timestamp(first),
        }
    }

    fn convert_timestamp(timezone_aware_datetime: DateTime<Utc>) -> i32 {
        let timestamp = timezone_aware_datetime.timestamp();
        i32::try_from(timestamp).map_or(i32::MAX, |val| val)
    }
}

impl fmt::Display for DateTimeCS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date_time = self.to_time();
        write!(f, "{:}", date_time)
    }
}

/// The (timezone-less) date and time that will be written in the archive alongside the file.
///
/// Use `FileDateTime::Zero` if the date and time are insignificant. This will set the value to 0 which is 1980, January 1th, 12AM.  
/// Use `FileDateTime::Custom` if you need to set a custom date and time.  
/// Use `FileDateTime::now()` if you want to use the current date and time (`chrono-datetime` feature required).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FileDateTime {
    /// MS-DOS origin time i.e. 1980, January 1th, 12AM.
    Zero,

    /// (year, month, day, hour, minute, second)
    Custom(DateTimeCS),

    ///
    Now,

    /// Current timestamp (seconds since UNIX epoch)
    UnixNow,

    /// Custom time in Unix format (seconds since UNIX epoch)
    UnixCustom(i32),
}

impl FileDateTime {
    fn tuple(&self) -> DateTimeCS {
        match self {
            FileDateTime::Zero => DateTimeCS::default(),
            FileDateTime::Custom(date_time) => *date_time,
            FileDateTime::Now => DateTimeCS::now(),
            FileDateTime::UnixNow => DateTimeCS::now(),
            FileDateTime::UnixCustom(timestamp) => DateTimeCS::from_timestamp(*timestamp),
        }
    }

    pub fn ms_dos(&self) -> (u16, u16) {
        self.tuple().ms_dos()
    }

    pub fn to_time(&self) -> chrono::NaiveDateTime {
        self.tuple().to_time()
    }

    pub fn timestamp(&self) -> i32 {
        match self {
            FileDateTime::Zero => DateTimeCS::default().to_timestamp(),
            FileDateTime::Custom(date_time) => date_time.to_timestamp(),
            FileDateTime::Now => DateTimeCS::convert_timestamp(chrono::offset::Utc::now()),
            FileDateTime::UnixNow => DateTimeCS::convert_timestamp(chrono::offset::Utc::now()),
            FileDateTime::UnixCustom(timestamp) => *timestamp,
        }
    }

    pub fn extended_timestamp(&self) -> bool {
        matches!(self, FileDateTime::UnixNow | FileDateTime::UnixCustom(_))
    }
}

impl Default for FileDateTime {
    /// Construct a new FileOptions object
    fn default() -> Self {
        FileDateTime::Zero
    }
}

/// Tells the compatibility system of the file attribute information.
///
/// Mapping as per [PKWARE's APPNOTE.TXT v6.3.10](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) section 4.4.2.1
#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum FileCompatibilitySystem {
    /// MS-DOS and OS/2 (FAT / VFAT / FAT32 file systems)
    Dos = 0,
    Unix = 3,
    WindowsNTFS = 10,
    OsX = 19,
    Unknown(u8),
}

impl FileCompatibilitySystem {
    pub fn from_u8(system_code: u8) -> FileCompatibilitySystem {
        use self::FileCompatibilitySystem::*;

        match system_code {
            0 => Dos,
            3 => Unix,
            10 => WindowsNTFS,
            19 => OsX,
            _ => Unknown(system_code),
        }
    }

    pub fn value(&self) -> u8 {
        match *self {
            FileCompatibilitySystem::Dos => 0,
            FileCompatibilitySystem::Unix => 3,
            FileCompatibilitySystem::WindowsNTFS => 10,
            FileCompatibilitySystem::OsX => 19,
            FileCompatibilitySystem::Unknown(val) => val,
        }
    }

    /// Add the system code to the version needed
    pub fn update_version_needed(&self, version_needed: u16) -> u16 {
        let val = self.value();

        (version_needed & 0xFF) | ((val as u16) << 8)
    }
}

impl fmt::Display for FileCompatibilitySystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            FileCompatibilitySystem::Dos => "MS-DOS, OS/2 or NT FAT".to_owned(),
            FileCompatibilitySystem::Unix => "Unix".to_owned(),
            FileCompatibilitySystem::WindowsNTFS => "Windows NTFS".to_owned(),
            FileCompatibilitySystem::OsX => "OsX".to_owned(),
            FileCompatibilitySystem::Unknown(val) => format!("unknown ({})", val),
        };

        write!(f, "{}", label)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_time_display() {
        let time: FileDateTime = FileDateTime::Zero;
        //  time.to_time
        let ctime = time.to_time();

        println!("Time zero {}", ctime)
    }

    #[test]
    fn test_time_display_zero_msdos() {
        let time: FileDateTime = FileDateTime::Zero;
        //  time.to_time

        let (date, time) = time.ms_dos();

        println!("Time zero {} {}", date, time)
    }

    #[test]
    fn test_time_display_0_0() {
        let date_time = DateTimeCS::from_msdos(0, 0);

        println!("Time zero {}", date_time)
    }

    #[test]
    fn test_time_display_time() {
        let time: FileDateTime = FileDateTime::Now;

        println!("Time zero {}", time.to_time());

        println!("{:?}", chrono::offset::Local::now());
        println!("{:?}", chrono::offset::Utc::now());
        let ts = chrono::offset::Utc::now().timestamp() as i32;
        println!("{:?}", ts);
    }

    #[test]
    fn test_file_compatibility_system() {
        assert_eq!(FileCompatibilitySystem::Dos.value(), 0);
        assert_eq!(FileCompatibilitySystem::Unix.value(), 3);
        assert_eq!(FileCompatibilitySystem::WindowsNTFS.value(), 10);
        assert_eq!(FileCompatibilitySystem::OsX.value(), 19);
        assert_eq!(FileCompatibilitySystem::Unknown(34).value(), 34);

        assert_eq!(
            FileCompatibilitySystem::from_u8(0),
            FileCompatibilitySystem::Dos
        );
        assert_eq!(
            FileCompatibilitySystem::from_u8(3),
            FileCompatibilitySystem::Unix
        );
        assert_eq!(
            FileCompatibilitySystem::from_u8(10),
            FileCompatibilitySystem::WindowsNTFS
        );
        assert_eq!(
            FileCompatibilitySystem::from_u8(19),
            FileCompatibilitySystem::OsX
        );
        assert_eq!(
            FileCompatibilitySystem::from_u8(55),
            FileCompatibilitySystem::Unknown(55)
        );
    }
}
