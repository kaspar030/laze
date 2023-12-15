use std::fs::File;
use std::process::Command;

use anyhow::{Context, Error};
use camino::{Utf8Path, Utf8PathBuf};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::download::{Download, Git, Source};
use crate::utils::calculate_hash;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Import {
    #[serde(flatten)]
    download: Download,
}

#[derive(RustEmbed)]
#[folder = "assets"]
struct Asset;

fn get_existing_file(path: &Utf8Path, filenames: &[&str]) -> Option<Utf8PathBuf> {
    for filename in filenames.iter() {
        let fullpath = path.join(filename);
        if path.join(filename).exists() {
            return Some(fullpath);
        }
    }
    None
}

impl Import {
    pub fn get_path<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        let source_hash = calculate_hash(&self.download);

        let mut res = Utf8PathBuf::from(build_dir.as_ref());
        res.push("imports");
        if let Some(name) = self.get_name() {
            res.push(format!("{name}-{source_hash}"));
        } else {
            res.push(format!("{source_hash}"));
        }
        Ok(res)
    }

    pub fn handle<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        let path = self.get_path(build_dir).unwrap();
        let tagfile = path.join(".laze-downloaded");

        if !tagfile.exists() {
            match &self.download.source {
                Source::Git(Git::Commit { url, commit }) => {
                    println!("IMPORT Git {url}:{commit} -> {path}");

                    let status = if Url::parse(url).is_ok() {
                        Command::new("git")
                            .args(["cache", "clone", url, commit, path.as_str()])
                            .status()?
                    } else {
                        let mut status = Command::new("git")
                            .args(["clone", "--no-checkout", url, path.as_str()])
                            .status()?;
                        if status.success() {
                            status = Command::new("git")
                                .args(["-C", path.as_str(), "checkout", commit])
                                .status()?;
                        }
                        status
                    };

                    if status.success() {
                        File::create(tagfile)?;
                    } else {
                        return Err(anyhow!(
                            "could not import from git url: {url} commit: {commit}",
                        ));
                    }
                }
                Source::Laze(name) => {
                    let mut at_least_one = false;
                    let prefix = format!("{name}/");
                    for filename in Asset::iter().filter(|x| x.starts_with(&prefix)) {
                        if !at_least_one {
                            at_least_one = true;
                            std::fs::create_dir_all(&path)
                                .with_context(|| format!("creating {path}"))?;
                        }

                        let embedded_file = Asset::get(&filename).unwrap();
                        let filename = filename.strip_prefix(&prefix).unwrap();
                        let filename = path.join(filename);
                        let parent = path.parent().unwrap();
                        std::fs::create_dir_all(path.parent().unwrap())
                            .with_context(|| format!("creating {parent}"))?;
                        std::fs::write(&filename, embedded_file.data)
                            .with_context(|| format!("creating {filename}"))?;
                    }
                    if at_least_one {
                        File::create(&tagfile).with_context(|| format!("creating {tagfile}"))?;
                    } else {
                        return Err(anyhow!("could not import from laze defaults: {name}"));
                    }
                }
            }
        }

        get_existing_file(&path, &["laze-lib.yml", "laze.yml", "laze-project.yml"]).ok_or(anyhow!(
            "no \"laze-lib.yml\", \"laze.yml\" or \"laze-project.yml\" in import"
        ))
    }

    pub fn get_name(&self) -> Option<String> {
        match &self.download.source {
            Source::Git(Git::Commit { url, .. }) => url.split('/').last().map(|x| x.to_string()),
            Source::Laze(name) => {
                let prefix = format!("{}/", name);

                if Asset::iter().any(|x| x.starts_with(&prefix)) {
                    let build_uuid_hash = calculate_hash(&build_uuid::get());
                    Some(format!("laze/{name}-{build_uuid_hash}"))
                } else {
                    None
                }
            } //_ => None,
        }
    }
}
