use std::fs::File;
use std::{fs, io};

#[test]
fn unzip_test() -> Result<(), std::io::Error> {
    let file = File::open("/tmp/zipstream/test_zlib_flate2.zip").unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    let fns = archive.file_names().collect::<String>();
    println!("files name, {:?}", fns);

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i).unwrap();
        let outpath = match zip_file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        {
            let comment = zip_file.comment();
            if !comment.is_empty() {
                println!("File {i} comment: {comment}");
            }
        }

        if (*zip_file.name()).ends_with('/') {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                zip_file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut zip_file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = zip_file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }

    Ok(())
}
