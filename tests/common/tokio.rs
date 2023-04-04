#![allow(dead_code)]
use ::std::path::Path;
use std::task::Poll;
use tokio::{fs::File, io::AsyncRead};

use super::PACKAGE_NAME;
const ENGINE: &str = "tokio";
const TEMP: &str = "/tmp";

pub async fn create_new_clean_file(file_name: &str) -> File {
    let out_dir = Path::new(TEMP).join(PACKAGE_NAME).join(ENGINE);
    if !out_dir.exists() {
        tokio::fs::create_dir_all(&out_dir)
            .await
            .unwrap_or_else(|error| {
                panic!("creating dir {:?} failed, because {:?}", out_dir, error);
            })
    }

    let out_path = out_dir.join(file_name);

    if out_path.exists() {
        tokio::fs::remove_file(&out_path)
            .await
            .unwrap_or_else(|error| {
                panic!("deleting file {:?} failed, because {:?}", &out_path, error);
            });
    }
    tokio::fs::File::create(&out_path)
        .await
        .unwrap_or_else(|error| {
            panic!("creating file {:?} failed, because {:?}", &out_path, error);
        })
}

/// Create
pub struct MockAsyncReader {
    mock_size: usize,
}

impl MockAsyncReader {
    pub fn new(size: usize) -> Self {
        Self { mock_size: size }
    }
}

impl AsyncRead for MockAsyncReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        //println!("Capacity {:?} ", buf.capacity());
        //println!("Remaining {:?}", buf.remaining());

        let size = buf.remaining().min(self.mock_size);
        buf.advance(size);

        for i in buf.filled_mut().iter_mut().take(size) {
            *i = b'0';
        }

        self.get_mut().mock_size -= size;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod test {
    use super::MockAsyncReader;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_reader() {
        let mut buf = [b'a'; 10];

        let mut reader = MockAsyncReader::new(105);

        loop {
            match reader.read(&mut buf).await {
                Ok(size) => {
                    if size == 0 {
                        println!("END of read");
                        break;
                    }
                    println!("{:?}", buf);
                    println!("Remaining  to read {:?}", reader.mock_size);
                    println!("Read {} bytes", size)
                }
                Err(e) => panic!("Error {:?}", e),
            }
        }
    }
}
