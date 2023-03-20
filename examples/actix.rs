use std::path::Path;

use actix_web::http::header::ContentDisposition;
use actix_web::{get, App, HttpResponse, HttpServer};
use archflow::compress::tokio::archive::ZipArchive;
use archflow::compress::FileOptions;
use archflow::compression::CompressionMethod;
use archflow::types::FileDateTime;
use tokio::fs::File;
use tokio::io::duplex;

use tokio_util::io::ReaderStream;

#[get("/zip")]
async fn zip_archive() -> HttpResponse {
    let (w, r) = duplex(4096);
    let options = FileOptions::default()
        .last_modified_time(FileDateTime::Now)
        .compression_method(CompressionMethod::Deflate());

    tokio::spawn(async move {
        let mut archive = ZipArchive::new_streamable_over_tcp(w);

        let p = Path::new("./tests/resources/lorem_ipsum.txt");
        let mut f = File::open(p).await.unwrap();
        archive
            .append("ipsum_deflate.txt", &options, &mut f)
            .await
            .unwrap();
        archive
            .append("file1.txt", &options, &mut b"world\n".as_ref())
            .await
            .unwrap();
        archive
            .append("file2.txt", &options, &mut b"world\n".as_ref())
            .await
            .unwrap();

        let options = options.compression_method(CompressionMethod::BZip2());
        archive
            .append("ipsum_bz.txt", &options, &mut f)
            .await
            .unwrap();

        let options = options.compression_method(CompressionMethod::Xz());
        archive
            .append("ipsum_xz.txt", &options, &mut f)
            .await
            .unwrap();

        archive.finalize().await.unwrap();
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/zip"))
        .insert_header(ContentDisposition::attachment("myzip.zip"))
        .streaming(ReaderStream::new(r))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let address = "127.0.0.1";
    let port = 8081;

    println!("Test url is http://{}:{}/zip", address, port);

    HttpServer::new(|| App::new().service(zip_archive))
        .bind((address, port))?
        .run()
        .await
}
