use std::io::Write;

#[derive(Debug)]
pub struct WriteWrapper<W: Write> {
    writer: W,
    written_bytes_count: u64,
}

pub trait BytesCounter {
    fn get_written_bytes_count(&self) -> u64;
    fn set_written_bytes_count(&mut self, count: u64);
}

impl<W: Write> WriteWrapper<W> {
    pub fn new(w: W) -> WriteWrapper<W> {
        Self {
            writer: w,
            written_bytes_count: 0,
        }
    }

    pub fn retrieve_writer(self) -> W {
        self.writer
    }
}

impl<W: Write> BytesCounter for WriteWrapper<W> {
    fn get_written_bytes_count(&self) -> u64 {
        self.written_bytes_count
    }

    fn set_written_bytes_count(&mut self, count: u64) {
        self.written_bytes_count = count;
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
