#![allow(dead_code)]

use super::PACKAGE_NAME;
const ENGINE: &str = "std";
const TEMP: &str = "/tmp";

use std::{
    fs::{create_dir_all, remove_file, File},
    io::Read,
    path::Path,
};

pub fn create_new_clean_file(file_name: &str) -> File {
    let out_dir = Path::new(TEMP).join(PACKAGE_NAME).join(ENGINE);
    if !out_dir.exists() {
        create_dir_all(&out_dir).unwrap_or_else(|error| {
            panic!("creating dir {:?} failed, because {:?}", &out_dir, error);
        })
    }

    let out_path = out_dir.join(file_name);

    if out_path.exists() {
        remove_file(&out_path).unwrap_or_else(|error| {
            panic!("deleting file {:?} failed, because {:?}", &out_path, error);
        });
    }
    File::create(&out_path).unwrap_or_else(|error| {
        panic!("creating file {:?} failed, because {:?}", &out_path, error);
    })
}

pub struct MockReader {
    mock_size: usize,
}

impl MockReader {
    pub fn new(size: usize) -> Self {
        Self { mock_size: size }
    }
}

impl Read for MockReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = buf.len().min(self.mock_size);
        buf.fill(b'0');
        /*        for i in buf.iter_mut().take(size) {
            *i = b'0';
        } */

        self.mock_size -= size;

        println!("{:0x}", self.mock_size);
        Ok(size)
    }
}

#[cfg(test)]
mod test {
    use super::MockReader;
    use std::io::Read;

    #[test]
    fn test_reader() {
        let mut buf = [b'a'; 10];

        let mut reader = MockReader::new(105);

        loop {
            match reader.read(&mut buf) {
                Ok(size) => {
                    if size == 0 {
                        break;
                    }
                    println!("{:?}", buf);
                    println!("Read {} bytes", size)
                }
                Err(e) => panic!("Error {:?}", e),
            }
        }
    }
}
