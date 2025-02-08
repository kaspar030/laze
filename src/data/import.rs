use anyhow::Error;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::utils::get_existing_file;

mod cmd;
mod download;
mod local;

#[derive(Debug, Serialize, Deserialize, Hash)]
#[serde(untagged)]
pub enum ImportEntry {
    Download(crate::download::Download),
    Command(cmd::Command),
    Local(local::Local),
}

impl ImportEntry {
    pub fn handle<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        match self {
            Self::Download(download) => download.handle(build_dir),
            Self::Command(command) => command.handle(build_dir),
            Self::Local(local) => local.handle(build_dir),
        }
    }
}

pub trait Import: std::hash::Hash {
    fn get_name(&self) -> Option<String>;
    fn get_dldir(&self) -> Option<&String>;
    fn handle<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error>;
    fn get_path<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        use crate::utils::calculate_hash;

        let source_hash = calculate_hash(&self);

        let mut res = Utf8PathBuf::from(build_dir.as_ref());
        res.push("imports");
        if let Some(dldir) = self.get_dldir() {
            res.push(dldir);
        } else if let Some(name) = self.get_name() {
            res.push(format!("{name}-{source_hash}"));
        } else {
            res.push(format!("{source_hash}"));
        }
        Ok(res)
    }
}

fn get_lazefile(path: &Utf8Path) -> Result<Utf8PathBuf, Error> {
    get_existing_file(
        path,
        &[
            "laze-lib.yml",
            "laze-lib.toml",
            "laze.yml",
            "laze.toml",
            "laze-project.yml",
            "laze-project.toml",
        ],
    )
    .ok_or(anyhow!(
        "no \"laze-lib.yml/toml\", \"laze.yml/toml\" or \"laze-project.yml/toml\" in import"
    ))
}
