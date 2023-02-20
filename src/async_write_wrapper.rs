use std::pin::Pin;
use tokio::io::AsyncWrite;

#[derive(Debug)]
pub struct AsyncWriteWrapper<W: AsyncWrite + Unpin> {
    writer: W,
    compress_length: usize,
}

impl<W: AsyncWrite + Unpin> AsyncWriteWrapper<W> {
    pub fn new(w: W) -> AsyncWriteWrapper<W> {
        Self {
            writer: w,
            compress_length: 0,
        }
    }

    pub fn get_compress_length(&self) -> usize {
        self.compress_length
    }

    pub fn retrieve_writer(self) -> W {
        self.writer
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
                wrapper.compress_length += nb_byte_written;
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
