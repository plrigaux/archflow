# Compstream


https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT


## Features
- Stream on the fly an archive from multiple AsyncRead objects.
- Single read / seek free implementation (the CRC and file size are calculated while streaming and are sent afterwards).
- [tokio](https://docs.rs/tokio/latest/tokio/io/index.html) `AsyncRead` / `AsyncWrite` compatible. 

Supported compression formats:
 - stored (i.e. none)
 - deflate
 - bzip2
 - zstd
 - xz
 - LZMA

## Limitations

- No zip64.

## Examples

- How to create a zip archive
- How to stream an aschive with Hyper
.
### [File system](examples/fs.rs)

### [Hyper](examples/hyper.rs)


## Disclaimer

This implementation is inspired by : 
 - https://github.com/scotow/zipit and
 - https://github.com/zip-rs/zip

