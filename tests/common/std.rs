#![allow(dead_code)]

use std::{
    fs::{create_dir_all, remove_file, File},
    path::Path,
};

pub fn create_new_clean_file(file_name: &str) -> File {
    let dir_prefix = "/tmp/zipstream/std";
    let out_dir = Path::new(dir_prefix);
    if !out_dir.exists() {
        create_dir_all(out_dir).unwrap_or_else(|error| {
            panic!("creating dir {:?} failed, because {:?}", dir_prefix, error);
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
