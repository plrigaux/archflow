use archflow::{
    archive::FileOptions, compress::tokio::archive::ZipArchive, compression::CompressionMethod,
    types::FileDateTime,
};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Request, Response, Server, StatusCode};
use std::io::Cursor;
use tokio::io::duplex;
use tokio_util::io::ReaderStream;

async fn zip_archive(_req: Request<Body>) -> Result<Response<Body>, hyper::http::Error> {
    let (filename_1, mut fd_1) = (String::from("file1.txt"), Cursor::new(b"hello\n".to_vec()));
    let (filename_2, mut fd_2) = (String::from("file2.txt"), Cursor::new(b"world\n".to_vec()));

    let (w, r) = duplex(4096);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflate())
        .last_modified_time(FileDateTime::Now);
    tokio::spawn(async move {
        let mut archive = ZipArchive::new_streamable(w);
        archive
            .append_file(&filename_1, &mut fd_1, &options)
            .await
            .unwrap();
        archive
            .append_file(&filename_2, &mut fd_2, &options)
            .await
            .unwrap();
        archive.finalize().await.unwrap();
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .body(Body::wrap_stream(ReaderStream::new(r)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = ([127, 0, 0, 1], 8080).into();
    let service =
        make_service_fn(|_| async { Ok::<_, hyper::http::Error>(service_fn(zip_archive)) });
    let server = Server::bind(&address).serve(service);

    println!("Listening on http://{}", address);
    server.await?;

    Ok(())
}
