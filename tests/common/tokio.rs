#![allow(dead_code)]
use ::std::path::Path;
use tokio::fs::File;

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
