use core::fmt;
use std::u16;

use crate::{compression::CompressionMethod, constants::VERSION_MADE_BY};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};

#[derive(Debug)]
pub struct ArchiveFileEntry {
    pub version_needed: u16,
    pub general_purpose_flags: u16,
    pub compression_method: u16,
    pub last_mod_file_time: u16,
    pub last_mod_file_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_len: u16,
    pub extra_field_length: u16,
    pub file_name_as_bytes: Vec<u8>,
    pub offset: u32,
    pub compressor: CompressionMethod,
}

impl ArchiveFileEntry {
    pub fn version_needed(&self) -> u16 {
        // higher versions matched first
        match self.compressor {
            CompressionMethod::BZip2() => 46,
            _ => 20,
        }
    }

    pub fn version_made_by(&self) -> u16 {
        VERSION_MADE_BY
    }

    fn extended_local_header(&self) -> bool {
        self.general_purpose_flags & (1u16 << 3) != 0
    }

    fn is_encrypted(&self) -> bool {
        self.general_purpose_flags & (1u16 << 0) != 0
    }

    fn pretty_version(zip_version: u16) -> (u16, u16) {
        let major = zip_version / 10;
        let minor = zip_version % 10;

        (major, minor)
    }
}

impl fmt::Display for ArchiveFileEntry {
    #[allow(clippy::writeln_empty_string)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let padding = 48;

        writeln!(
            f,
            "{: <padding$}{}",
            "offset of local header from start of archive:", self.offset
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

        let date_time = DateTimeCS::from_msdos(self.last_mod_file_date, self.last_mod_file_date);
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

        writeln!(f, "")
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

    pub fn timestamp(&self) -> i32 {
        let local = &self.to_time();

        match local.and_local_timezone(Utc) {
            chrono::LocalResult::None => todo!(),
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
    /// 1980, January 1th, 12AM.
    Zero,
    /// (year, month, day, hour, minute, second)
    Custom(DateTimeCS),
    Now,
}

impl FileDateTime {
    fn tuple(&self) -> DateTimeCS {
        match self {
            FileDateTime::Zero => DateTimeCS::default(),
            FileDateTime::Custom(date_time) => *date_time,
            FileDateTime::Now => DateTimeCS::now(),
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
            FileDateTime::Zero => DateTimeCS::default().timestamp(),
            FileDateTime::Custom(date_time) => date_time.timestamp(),
            FileDateTime::Now => DateTimeCS::convert_timestamp(chrono::offset::Utc::now()),
        }
    }
}

impl Default for FileDateTime {
    /// Construct a new FileOptions object
    fn default() -> Self {
        FileDateTime::Zero
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
}
