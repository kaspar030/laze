use std::path::{Path, PathBuf};

pub fn path_clone_push(path: &Path, append: &str) -> PathBuf {
    let mut res = PathBuf::from(path);
    res.push(append);
    res
}
