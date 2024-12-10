use std::fs::remove_dir_all;

use anyhow::{Context as _, Error};
use camino::{Utf8Path, Utf8PathBuf};
use git_cache::GitCacheClonerBuilder;
use rust_embed::RustEmbed;

use super::Import;
use crate::download::{Download, Git, Source};

#[derive(RustEmbed)]
#[folder = "assets/imports"]
struct Asset;

fn git_cloner(url: &str, target_path: &Utf8Path) -> GitCacheClonerBuilder {
    let git_cache = crate::GIT_CACHE.get().expect("this has been set earlier");

    let mut git_cache_builder = git_cache.cloner();

    git_cache_builder
        .repository_url(url.to_string())
        .target_path(Some(target_path.to_path_buf()));

    git_cache_builder
}

fn git_clone_commit(url: &str, target_path: &Utf8Path, commit: &str) -> Result<(), Error> {
    git_cloner(url, target_path)
        .commit(Some(commit.into()))
        .do_clone()
}

fn git_clone_branch(url: &str, target_path: &Utf8Path, branch: &str) -> Result<(), Error> {
    git_cloner(url, target_path)
        .update(true)
        .extra_clone_args(Some(vec!["--branch".into(), branch.into()]))
        .do_clone()
}

impl Import for Download {
    fn handle<T: AsRef<Utf8Path>>(&self, build_dir: T) -> Result<Utf8PathBuf, Error> {
        let target_path = self.get_path(build_dir).unwrap();
        let tagfile = target_path.join(".laze-downloaded");

        let mut skip_download = false;
        if tagfile.exists() {
            skip_download = self.compare_with_tagfile(&tagfile).unwrap_or_default();
        }
        if !skip_download {
            if target_path.exists() {
                remove_dir_all(&target_path)
                    .with_context(|| format!("removing path \"{target_path}\""))?;
            }

            match &self.source {
                Source::Git(Git::Commit { url, commit }) => {
                    println!("IMPORT Git {url}:{commit} -> {target_path}");

                    git_clone_commit(url, &target_path, commit).with_context(|| {
                        format!("cloning git url: \"{url}\" commit: \"{commit}\"")
                    })?;

                    self.create_tagfile(tagfile)?;
                }
                Source::Git(Git::Branch {
                    url,
                    branch: branch_or_tag,
                })
                | Source::Git(Git::Tag {
                    url,
                    tag: branch_or_tag,
                }) => {
                    println!("IMPORT Git {url}:{branch_or_tag} -> {target_path}");

                    git_clone_branch(url, &target_path, branch_or_tag).with_context(|| {
                        format!("cloning git url: \"{url}\" branch/tag: \"{branch_or_tag}\"")
                    })?;

                    self.create_tagfile(tagfile)?;
                }
                Source::Git(Git::Default { url }) => {
                    println!("IMPORT Git {url} -> {target_path}");

                    git_cloner(url, &target_path)
                        .do_clone()
                        .with_context(|| format!("cloning git url: \"{url}\""))?;

                    self.create_tagfile(tagfile)?;
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
                        self.create_tagfile(&tagfile)
                            .with_context(|| format!("creating {tagfile}"))?;
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
            Source::Git(Git::Commit { url, .. })
            | Source::Git(Git::Branch { url, .. })
            | Source::Git(Git::Tag { url, .. })
            | Source::Git(Git::Default { url, .. }) => url.split('/').last().map(|x| x.to_string()),
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
