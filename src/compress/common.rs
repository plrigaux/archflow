/// Fast routine for detection of plain text
///  (ASCII or an ASCII-compatible extension such as ISO-8859, UTF-8, etc.)
/// Author: Cosmin Truta.
///
/// See "proginfo/txtvsbin.txt" for more information.
pub fn is_text_buf(buffer: &[u8]) -> bool {
    println!("BUF LEN {}", buffer.len());
    let mut result = false;
    for c in buffer {
        if *c >= 32 {
            result = true;
        } else if (*c <= 6) || (*c >= 14 && *c <= 25) || (*c >= 28 && *c <= 31) {
            return false; // black-listed character found; stop
        }
    }
    result
}

/* macro_rules! decode {
    ($reader:ident $($_await:tt)*) => {
        {
            let mut i = 0u64;
            let mut buf = [0u8; 1];

            let mut j = 0;
            loop {
                if j > 9 {
                    // if j * 7 > 64
                    panic!() // todo move to an error
                }
                $reader.read_exact(&mut buf[..])$($_await)*?;
                i |= (u64::from(buf[0] & 0x7F)) << (j * 7);
                if (buf[0] >> 7) == 0 {
                    break;
                } else {
                    j += 1;
                }
            }

            Ok(i)
        }
    }
}

async fn decode_variable_async<R: AsyncRead + Unpin + Send>(reader: &mut R) -> Result<u64> {
    decode!(reader.await)
}

async fn decode_variable<R: Read>(reader: &mut R) -> Result<u64> {
    decode!(reader)
}
*/
#[cfg(test)]
mod test {
    use super::is_text_buf;

    #[test]
    fn all_text() {
        let res = is_text_buf(b"Some string data");

        assert!(res)
    }

    #[test]
    fn not_all_text() {
        let mut v = b"Some string data".to_vec();
        v.push(3u8);

        let res = is_text_buf(&v);

        assert!(!res)
    }
}
