use anyhow::{anyhow, Context, Error};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{data::ImportRoot, serde_bool_helpers::default_as_false};

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Local {
    name: Option<String>,
    path: Utf8PathBuf,
    dldir: Option<String>,
    #[serde(default = "default_as_false")]
    symlink: bool,
}

#[derive(Debug, Hash)]
pub struct LocalRelative<'a> {
    pub local: &'a Local,
    pub import_root: Option<&'a ImportRoot>,
}

impl<'a> super::Import for LocalRelative<'a> {
    fn get_path<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        let mut res = Utf8PathBuf::from(build_dir.as_ref());
        res.push("imports");
        if let Some(dldir) = self.get_dldir() {
            res.push(dldir);
        } else if let Some(name) = self.get_name() {
            res.push(name);
        } else {
            res.push(self.local.path.file_name().unwrap());
        }
        Ok(res)
    }

    fn get_name(&self) -> Option<String> {
        self.local.name.clone()
    }

    fn get_dldir(&self) -> Option<&String> {
        self.local.dldir.as_ref()
    }

    fn handle<T: AsRef<camino::Utf8Path>>(
        &self,
        build_dir: T,
    ) -> Result<camino::Utf8PathBuf, anyhow::Error> {
        let import_root = match self.import_root {
            Some(root) => root.path().canonicalize_utf8()?,
            None => ".".into(),
        };
        let target_path = import_root.join(&self.local.path);
        let target_path_canonical = target_path
            .canonicalize_utf8()
            .map_err(|e| anyhow!(format!("Path {target_path} is invalid: {e}?")))?;

        if self.local.symlink {
            let path = self.get_path(&build_dir)?;
            let path_parent = path.parent().unwrap();
            std::fs::create_dir_all(path_parent).with_context(|| format!("creating {path}"))?;

            let link_target = if target_path.is_relative() {
                let relative = pathdiff::diff_utf8_paths(&target_path, path_parent).unwrap();
                // If the symlink is itself accessed via a symlink, then the final symlink's target resolves relative to the real directory of the intermediate symlink, not the current directory.
                // This may happen if build/ is symlinked (e.g. to save space).
                let canonical_path = self.get_path(build_dir.as_ref().canonicalize_utf8()?)?;
                let from_relative = canonical_path.join(&relative).canonicalize_utf8();
                // An error means the path is not valid already.
                // std::io::Error does not support PartialEq, so map it to ()
                if from_relative.as_ref().map_err(|_| ()) != Ok(&target_path_canonical) {
                    target_path_canonical.clone()
                } else {
                    relative
                }
            } else {
                target_path_canonical.clone()
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
                    .with_context(|| format!("importing path {}", target_path_canonical))?;
            }
            // using `path` here as that is the path relative to the project root.
            super::get_lazefile(&path)
        } else {
            super::get_lazefile(&target_path_canonical)
        }
    }
}
