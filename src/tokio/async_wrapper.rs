use std::pin::Pin;
use tokio::io::AsyncWrite;

#[derive(Debug)]
pub struct AsyncWriteWrapper<W: AsyncWrite + Unpin> {
    writer: W,
    written_bytes_count: u64,
}

pub trait BytesCounter {
    fn get_written_bytes_count(&self) -> u64;
    fn set_written_bytes_count(&mut self, count: u64);
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
    fn get_written_bytes_count(&self) -> u64 {
        self.written_bytes_count
    }

    fn set_written_bytes_count(&mut self, count: u64) {
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
