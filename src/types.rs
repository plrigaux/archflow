use core::fmt;
use std::u16;

use crate::{archive::VERSION_MADE_BY, compression::Compressor};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike};

#[derive(Debug)]
pub struct ArchiveFileEntry {
    pub version_needed: u16,
    pub general_purpose_flags: u16,
    pub compression_method: u16,
    pub last_mod_file_time: u16,
    pub last_mod_file_date: u16,
    pub crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_len: u16,
    pub extra_field_length: u16,
    pub file_name_as_bytes: Vec<u8>,
    pub offset: u32,
    pub compressor: Compressor,
}

impl ArchiveFileEntry {
    pub fn version_needed(&self) -> u16 {
        // higher versions matched first
        match self.compressor {
            Compressor::BZip2() => 46,
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
    /* offset of local header from start of archive:   0
    (0000000000000000h) bytes
    file system or operating system of origin:      Unix
    version of encoding software:                   4.6
    minimum file system compatibility required:     MS-DOS, OS/2 or NT FAT
    minimum software version required to extract:   2.0
    compression method:                             deflated
    compression sub-type (deflation):               normal
    file security status:                           not encrypted
    extended local header:                          no
    file last modified on (DOS date/time):          1980 000 0 00:00:00
    32-bit CRC value (hex):                         b3b7851d
    compressed size:                                14022 bytes
    uncompressed size:                              4120799 bytes
    length of filename:                             9 characters
    length of extra field:                          0 bytes
    length of file comment:                         0 characters
    disk number on which file begins:               disk 1
    apparent file type:                             binary
    Unix file attributes (100644 octal):            -rw-r--r--
    MS-DOS file attributes (00 hex):                none

    There is no file comment. */
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

        let compressor = Compressor::from_compression_method(self.compression_method);
        let label = if compressor.is_unknown() {
            let str_val = self.compression_method.to_string();

            let mut val = String::from(compressor.compression_method_label());
            val.push_str(" (");
            val.push_str(&str_val);
            val.push(')');
            val
        } else {
            compressor.compression_method_label().to_owned()
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

        let date_time = FileDateTime::from_msdos(self.last_mod_file_date, self.last_mod_file_date);
        writeln!(
            f,
            "{: <padding$}{}",
            "file last modified on (DOS date/time):", date_time
        )?;

        writeln!(f, "{: <padding$}{:x}", "32-bit CRC value (hex):", self.crc)?;

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
    Custom {
        year: u16,
        month: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
    },
}

impl FileDateTime {
    fn tuple(&self) -> (u16, u16, u16, u16, u16, u16) {
        match self {
            FileDateTime::Zero => Default::default(),
            &FileDateTime::Custom {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => (year, month, day, hour, minute, second),
        }
    }

    pub fn ms_dos(&self) -> (u16, u16) {
        let (year, month, day, hour, min, sec) = self.tuple();
        (
            day | month << 5 | year.saturating_sub(1980) << 9,
            (sec / 2) | min << 5 | hour << 11,
        )
    }

    /// Use the local date and time of the system.
    pub fn now() -> Self {
        Self::from_chrono_datetime(Local::now())
    }

    /// Use a custom date and time.
    pub fn from_chrono_datetime<Tz: TimeZone>(datetime: DateTime<Tz>) -> Self {
        Self::Custom {
            year: datetime.year() as u16,
            month: datetime.month() as u16,
            day: datetime.day() as u16,
            hour: datetime.hour() as u16,
            minute: datetime.minute() as u16,
            second: datetime.second() as u16,
        }
    }

    pub fn from_msdos(datepart: u16, timepart: u16) -> Self {
        let seconds = (timepart & 0b0000000000011111) << 1;
        let minutes = (timepart & 0b0000011111100000) >> 5;
        let hours = (timepart & 0b1111100000000000) >> 11;
        let days = datepart & 0b0000000000011111;
        let months = (datepart & 0b0000000111100000) >> 5;
        let years = (datepart & 0b1111111000000000) >> 9;

        Self::Custom {
            year: years + 1980,
            month: months,
            day: days,
            hour: hours,
            minute: minutes,
            second: seconds,
        }
    }

    pub fn to_time(&self) -> chrono::NaiveDateTime {
        //println!("to_time {:?}", self);
        match self {
            FileDateTime::Custom {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => FileDateTime::to_time_dry(
                *year as i32,
                *month as u32,
                *day as u32,
                *hour as u32,
                *minute as u32,
                *second as u32,
            ),
            _ => {
                let dt = FileDateTime::from_msdos(0u16, 0u16);
                dt.to_time()
            }
        }
    }

    fn to_time_dry(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> chrono::NaiveDateTime {
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1980, 1, 1).unwrap());

        date.and_hms_opt(hour, minute, second).unwrap_or_default()
    }
}

impl fmt::Display for FileDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date_time = self.to_time();
        write!(f, "{:}", date_time)
    }
}

#[cfg(test)]
mod test {
    use super::FileDateTime;

    #[test]
    fn test_time_display() {
        let time: FileDateTime = FileDateTime::Zero;
        //  time.to_time
        let ctime = time.to_time();

        println!("Time zero {}", ctime)
    }

    #[test]
    fn test_time_display_0_0() {
        let date_time = FileDateTime::from_msdos(0, 0);

        println!("Time zero {}", date_time)
    }
}
