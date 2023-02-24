use std::path::Path;
use tokio::fs::File;

pub async fn create_new_clean_file(file_name: &str) -> File {
    let dir_prefix = "/tmp/zipstream";
    let out_dir = Path::new(dir_prefix);
    if !out_dir.exists() {
        tokio::fs::create_dir_all(out_dir)
            .await
            .unwrap_or_else(|error| {
                panic!("creating dir {:?} failed, because {:?}", dir_prefix, error);
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
    let file = tokio::fs::File::create(&out_path)
        .await
        .unwrap_or_else(|error| {
            panic!("creating file {:?} failed, because {:?}", &out_path, error);
        });

    file
}

