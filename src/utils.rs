use std::path::{Path, PathBuf};

pub fn path_from_uri(uri: String) -> PathBuf {
    if uri.starts_with("file://") {
        Path::new(uri.split_once("file://").unwrap().1).to_path_buf()
    } else {
        Path::new(&uri).to_path_buf()
    }
}
