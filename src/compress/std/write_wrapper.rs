use std::{
    fmt::Debug,
    io::{Error, Seek, Write},
};

#[derive(Debug)]
pub struct WriteWrapper<W: Write> {
    writer: W,
    written_bytes_count: u64,
}

#[derive(Debug)]
pub struct WriteSeekWrapper<WS: Write + Seek> {
    writer: WS,
    written_bytes_count: u64,
}

pub trait CommonWrapper<W: Write + ?Sized>: Write + Seek {
    fn get_written_bytes_count(&mut self) -> Result<u64, Error>;
    fn set_written_bytes_count(&mut self, count: u64);
    fn get_into(self: Box<Self>) -> W;
}

impl<W: Write> CommonWrapper<W> for WriteWrapper<W> {
    fn get_written_bytes_count(&mut self) -> Result<u64, Error> {
        Ok(self.written_bytes_count)
    }

    fn set_written_bytes_count(&mut self, count: u64) {
        self.written_bytes_count = count;
    }

    fn get_into(self: Box<Self>) -> W {
        self.writer
    }
}

impl<W: Write> WriteWrapper<W> {
    pub fn new(w: W) -> WriteWrapper<W> {
        Self {
            writer: w,
            written_bytes_count: 0,
        }
    }

    pub fn get_into(self: Box<Self>) -> W {
        self.writer
    }
}

impl<W: Write> Seek for WriteWrapper<W> {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Ok(self.written_bytes_count)
    }
}

impl<W: Write> Write for WriteWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.writer.write(buf) {
            Ok(nb_byte_written) => {
                self.written_bytes_count += nb_byte_written as u64;
                Ok(nb_byte_written)
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> WriteSeekWrapper<W> {
    pub fn new(w: W) -> WriteSeekWrapper<W> {
        Self {
            writer: w,
            written_bytes_count: 0,
        }
    }

    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self.writer.seek(pos) {
            Ok(position_from_start) => {
                self.written_bytes_count = position_from_start;
                Ok(position_from_start)
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_into(self) -> W {
        self.writer
    }
}

impl<W: Write + Seek> Write for WriteSeekWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for WriteSeekWrapper<W> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        WriteSeekWrapper::seek(self, pos)
    }
}

impl<W: Write + Seek> CommonWrapper<W> for WriteSeekWrapper<W> {
    fn get_written_bytes_count(&mut self) -> Result<u64, Error> {
        WriteSeekWrapper::seek(self, std::io::SeekFrom::Current(0))
    }

    fn set_written_bytes_count(&mut self, count: u64) {
        self.written_bytes_count = count;
    }

    fn get_into(self: Box<Self>) -> W {
        self.writer
    }
}
