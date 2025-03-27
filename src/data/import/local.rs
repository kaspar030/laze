use anyhow::{anyhow, Context, Error};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::serde_bool_helpers::default_as_false;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Local {
    name: Option<String>,
    path: Utf8PathBuf,
    dldir: Option<String>,
    #[serde(default = "default_as_false")]
    symlink: bool,
}

impl super::Import for Local {
    fn get_path<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        let mut res = Utf8PathBuf::from(build_dir.as_ref());
        res.push("imports");
        if let Some(dldir) = self.get_dldir() {
            res.push(dldir);
        } else if let Some(name) = self.get_name() {
            res.push(name);
        } else {
            res.push(self.path.file_name().unwrap());
        }
        Ok(res)
    }

    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    fn get_dldir(&self) -> Option<&String> {
        self.dldir.as_ref()
    }

    fn handle<T: AsRef<camino::Utf8Path>>(
        &self,
        build_dir: T,
    ) -> Result<camino::Utf8PathBuf, anyhow::Error> {
        if self.symlink {
            let path = self.get_path(&build_dir)?;

            let path_parent = path.parent().unwrap();
            std::fs::create_dir_all(path_parent).with_context(|| format!("creating {path}"))?;

            let link_target = if self.path.is_relative() {
                pathdiff::diff_utf8_paths(&self.path, path_parent).unwrap()
            } else {
                self.path.clone()
            };

            let mut link_is_missing = true;
            if path.is_symlink() {
                if path.read_link().unwrap() == link_target {
                    link_is_missing = false;
                } else {
                    std::fs::remove_file(&path).with_context(|| format!("removing {path}"))?;
                }
            } else if path.exists() {
                return Err(anyhow!("import target {path} exists and is not empty!"));
            }

            if link_is_missing {
                #[cfg(target_family = "windows")]
                let res = std::os::windows::fs::symlink_dir(&link_target, &path);
                #[cfg(target_family = "unix")]
                let res = std::os::unix::fs::symlink(&link_target, &path);

                res.with_context(|| format!("creating symlink {link_target}"))
                    .with_context(|| format!("importing path {}", self.path))?;
            }
            // using `path` here as that is the path relative to the project root.
            super::get_lazefile(&path)
        } else {
            super::get_lazefile(&self.path)
        }
    }
}
