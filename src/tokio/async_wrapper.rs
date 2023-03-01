use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncSeek, AsyncWrite};

#[derive(Debug)]
pub struct AsyncWriteWrapper<W: AsyncWrite + Unpin> {
    writer: W,
    written_bytes_count: usize,
}

pub trait BytesCounter {
    fn get_written_bytes_count(&self) -> usize;
    fn set_written_bytes_count(&mut self, count: usize);
}

impl<W: AsyncWrite + Unpin> AsyncWriteWrapper<W> {
    pub fn new(w: W) -> AsyncWriteWrapper<W> {
        Self {
            writer: w,
            written_bytes_count: 0,
        }
    }

    pub fn retrieve_writer(self) -> W {
        self.writer
    }
}

impl<W: AsyncWrite + Unpin> BytesCounter for AsyncWriteWrapper<W> {
    fn get_written_bytes_count(&self) -> usize {
        self.written_bytes_count
    }

    fn set_written_bytes_count(&mut self, count: usize) {
        self.written_bytes_count = count;
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for AsyncWriteWrapper<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let wrapper = self.get_mut();
        let results: std::task::Poll<Result<usize, std::io::Error>> =
            Pin::new(&mut wrapper.writer).poll_write(cx, buf);

        results.map(|pool_result| match pool_result {
            Ok(nb_byte_written) => {
                wrapper.written_bytes_count += nb_byte_written;
                Ok(nb_byte_written)
            }
            Err(e) => Err(e),
        })
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_shutdown(cx)
    }
}

#[derive(Debug)]
pub struct AsyncWriteSeekWrapper<W: AsyncWrite + AsyncSeek + Unpin> {
    writer_seek: W,
    written_bytes_count: usize,
}

impl<W: AsyncWrite + AsyncSeek + Unpin> AsyncWriteSeekWrapper<W> {
    pub fn new(w: W) -> AsyncWriteSeekWrapper<W> {
        Self {
            writer_seek: w,
            written_bytes_count: 0,
        }
    }

    pub fn get_written_bytes_count(&self) -> usize {
        self.written_bytes_count
    }

    pub fn retrieve_writer(self) -> W {
        self.writer_seek
    }
}

impl<W: AsyncWrite + AsyncSeek + Unpin> AsyncWrite for AsyncWriteSeekWrapper<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let wrapper = self.get_mut();
        let results: std::task::Poll<Result<usize, std::io::Error>> =
            Pin::new(&mut wrapper.writer_seek).poll_write(cx, buf);

        results.map(|pool_result| match pool_result {
            Ok(nb_byte_written) => {
                wrapper.written_bytes_count += nb_byte_written;
                Ok(nb_byte_written)
            }
            Err(e) => Err(e),
        })
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().writer_seek).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().writer_seek).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        let buf = bufs
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        Pin::new(&mut self.get_mut().writer_seek).poll_write(cx, buf)
    }

    fn is_write_vectored(&self) -> bool {
        self.writer_seek.is_write_vectored()
    }
}

impl<W: AsyncWrite + AsyncSeek + Unpin> AsyncSeek for AsyncWriteSeekWrapper<W> {
    fn start_seek(self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        Pin::new(&mut self.get_mut().writer_seek).start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Pin::new(&mut self.get_mut().writer_seek).poll_complete(cx)
    }
}

impl<W: AsyncWrite + AsyncSeek + Unpin> BytesCounter for AsyncWriteSeekWrapper<W> {
    fn get_written_bytes_count(&self) -> usize {
        self.written_bytes_count
    }

    fn set_written_bytes_count(&mut self, count: usize) {
        self.written_bytes_count = count;
    }
}
