use std::fs::File;

use anyhow::{Context as _, Error};
use camino::{Utf8Path, Utf8PathBuf};
use rust_embed::RustEmbed;

use super::Import;
use crate::download::{Download, Git, Source};

#[derive(RustEmbed)]
#[folder = "assets/imports"]
struct Asset;

fn git_clone(url: &str, target_path: &Utf8Path, commit: &str) -> Result<(), Error> {
    let git_cache = crate::GIT_CACHE.get().expect("this has been set earlier");

    git_cache
        .cloner()
        .repository_url(url.to_string())
        .target_path(Some(target_path.to_path_buf()))
        .commit(Some(commit.into()))
        .do_clone()
}

impl Import for Download {
    fn handle<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        let target_path = self.get_path(build_dir).unwrap();
        let tagfile = target_path.join(".laze-downloaded");

        if !tagfile.exists() {
            match &self.source {
                Source::Git(Git::Commit { url, commit }) => {
                    println!("IMPORT Git {url}:{commit} -> {target_path}");

                    git_clone(url, &target_path, commit).with_context(|| {
                        format!("cloning git url: \"{url}\" commit: \"{commit}\"")
                    })?;

                    File::create(tagfile)?;
                }
                Source::Laze(name) => {
                    let mut at_least_one = false;
                    let prefix = format!("{name}/");
                    for filename in Asset::iter().filter(|x| x.starts_with(&prefix)) {
                        if !at_least_one {
                            at_least_one = true;
                            std::fs::create_dir_all(&target_path)
                                .with_context(|| format!("creating {target_path}"))?;
                        }

                        let embedded_file = Asset::get(&filename).unwrap();
                        let filename = filename.strip_prefix(&prefix).unwrap();
                        let filename = target_path.join(filename);
                        let parent = target_path.parent().unwrap();
                        std::fs::create_dir_all(target_path.parent().unwrap())
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

        super::get_lazefile(&target_path)
    }

    fn get_dldir(&self) -> Option<&String> {
        self.dldir.as_ref()
    }

    fn get_name(&self) -> Option<String> {
        use crate::utils::calculate_hash;

        match &self.source {
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
