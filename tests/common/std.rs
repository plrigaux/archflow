#![allow(dead_code)]

use super::PACKAGE_NAME;
const ENGINE: &str = "std";
const TEMP: &str = "/tmp";

use std::{
    fs::{create_dir_all, remove_file, File},
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
