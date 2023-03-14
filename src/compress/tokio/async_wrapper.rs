use std::io::Error;
use std::pin::Pin;
use tokio::io::{AsyncSeek, AsyncWrite};

pub struct AsyncWriteWrapper<W: AsyncWrite + Unpin> {
    writer: W,
    written_bytes_count: u64,
}

pub struct AsyncWriteSeekWrapper<WS: AsyncWrite + AsyncSeek + Unpin> {
    writer: WS,
    written_bytes_count: u64,
}

pub trait CommonWrapper<W: AsyncWrite + Unpin + ?Sized>:
    AsyncWrite + AsyncSeek + Unpin + std::marker::Send
{
    fn get_written_bytes_count(&mut self) -> Result<u64, Error>;
    fn set_written_bytes_count(&mut self, count: u64);
    fn get_into(self: Box<Self>) -> W;
}

impl<W: AsyncWrite + Unpin + Send> CommonWrapper<W> for AsyncWriteWrapper<W> {
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

impl<W: AsyncWrite + AsyncSeek + Unpin + Send> CommonWrapper<W> for AsyncWriteSeekWrapper<W> {
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

impl<W: AsyncWrite + Unpin> AsyncWriteWrapper<W> {
    pub fn new(w: W) -> AsyncWriteWrapper<W> {
        Self {
            writer: w,
            written_bytes_count: 0,
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncSeek for AsyncWriteWrapper<W> {
    fn start_seek(self: Pin<&mut Self>, _position: std::io::SeekFrom) -> std::io::Result<()> {
        Ok(())
    }

    fn poll_complete(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        std::task::Poll::Ready(Ok(self.written_bytes_count))
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
                wrapper.written_bytes_count += nb_byte_written as u64;
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

impl<W: AsyncWrite + AsyncSeek + Unpin> AsyncWriteSeekWrapper<W> {
    pub fn new(w: W) -> AsyncWriteSeekWrapper<W> {
        Self {
            writer: w,
            written_bytes_count: 0,
        }
    }
}

impl<W: AsyncWrite + AsyncSeek + Unpin> AsyncWrite for AsyncWriteSeekWrapper<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_write(cx, buf)
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

impl<W: AsyncWrite + AsyncSeek + Unpin> AsyncSeek for AsyncWriteSeekWrapper<W> {
    fn start_seek(self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        Pin::new(&mut self.get_mut().writer).start_seek(position)
    }

    fn poll_complete(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        Pin::new(&mut self.get_mut().writer).poll_complete(cx)
    }
}
