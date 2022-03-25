use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Error;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::download::{Download, Git, Source};
use crate::utils::calculate_hash;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Import {
    #[serde(flatten)]
    download: Download,
}

fn get_existing_file(path: &Path, filenames: &[&str]) -> Option<PathBuf> {
    for filename in filenames.iter() {
        let fullpath = path.join(filename);
        if path.join(filename).exists() {
            return Some(PathBuf::from(fullpath));
        }
    }
    None
}

impl Import {
    pub fn get_path<T: AsRef<Path>>(&self, build_dir: T) -> Result<PathBuf, Error> {
        let source_hash = calculate_hash(&self.download);

        let mut res = PathBuf::from(build_dir.as_ref());
        res.push("imports");
        if let Some(name) = self.get_name() {
            res.push(format!("{}-{}", name, source_hash));
        } else {
            res.push(format!("{}", source_hash));
        }
        Ok(res)
    }

    pub fn handle<T: AsRef<Path>>(&self, build_dir: T) -> Result<PathBuf, Error> {
        let path = self.get_path(build_dir).unwrap();
        let tagfile = path.join(".laze-downloaded");

        if !tagfile.exists() {
            match &self.download.source {
                Source::Git(Git::Commit { url, commit }) => {
                    println!("IMPORT Git {}:{} -> {:?}", &url, commit, path);

                    let status = if Url::parse(&url).is_ok() {
                        Command::new("git")
                            .args([
                                "cache",
                                "clone",
                                &url,
                                commit,
                                path.as_os_str().to_str().unwrap(),
                            ])
                            .status()?
                    } else {
                        let mut status = Command::new("git")
                            .args([
                                "clone",
                                "--no-checkout",
                                &url,
                                path.as_os_str().to_str().unwrap(),
                            ])
                            .status()?;
                        if status.success() {
                            status = Command::new("git")
                                .args([
                                    "-C",
                                    path.as_os_str().to_str().unwrap(),
                                    "checkout",
                                    &commit,
                                ])
                                .status()?;
                        }
                        status
                    };

                    if status.success() {
                        File::create(tagfile)?;
                    } else {
                        return Err(anyhow!(
                            "could not import from git url: {} commit: {}",
                            url,
                            commit
                        ));
                    }
                }
            }
        }

        if let Some(laze_file) =
            get_existing_file(&path, &["laze-lib.yml", "laze.yml", "laze-project.yml"])
        {
            return Ok(laze_file);
        } else {
            return Err(anyhow!(
                "no \"laze-lib.yml\", \"laze.yml\" or \"laze-project.yml\" in import"
            ));
        }
    }

    pub fn get_name(&self) -> Option<&str> {
        match &self.download.source {
            Source::Git(Git::Commit { url, .. }) => url.split("/").last(),
            //_ => None,
        }
    }
}
