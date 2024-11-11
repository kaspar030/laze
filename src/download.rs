//! This module deals with "download:" directives

use std::borrow::Cow;

use anyhow::Result;
use im::HashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::{ninja::NinjaBuildBuilder, Module, Rule};
use crate::nested_env::{self, IfMissing};
use camino::{Utf8Path, Utf8PathBuf};

pub mod source {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
    pub enum Source {
        #[serde(rename = "git")]
        Git(Git),
        #[serde(rename = "laze")]
        Laze(String),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
    #[serde(untagged)]
    pub enum Git {
        Commit { url: String, commit: String },
        Branch { url: String, branch: String },
        Tag { url: String, tag: String },
        Default { url: String },
    }
}

pub use source::{Git, Source};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct Download {
    #[serde(flatten)]
    pub source: source::Source,
    pub patches: Option<Vec<String>>,
    pub dldir: Option<String>,
}

impl Download {
    pub fn srcdir(&self, build_dir: &Utf8Path, module: &Module) -> Utf8PathBuf {
        let mut srcdir = Utf8PathBuf::from(build_dir);
        srcdir.push("dl");
        if let Some(dldir) = &self.dldir {
            srcdir.push(dldir);
        } else {
            srcdir.push(module.relpath.as_ref().unwrap().clone());
            srcdir.push(module.name.clone());
        }
        srcdir
    }

    fn tagfile_download(&self, srcdir: &Utf8PathBuf) -> Utf8PathBuf {
        Utf8Path::new(srcdir).join(".laze-downloaded")
    }

    fn tagfile_patched(&self, srcdir: &Utf8PathBuf) -> Utf8PathBuf {
        Utf8Path::new(srcdir).join(".laze-patched")
    }

    pub fn tagfile(&self, srcdir: &Utf8PathBuf) -> Utf8PathBuf {
        if self.patches.is_some() {
            self.tagfile_patched(srcdir)
        } else {
            self.tagfile_download(srcdir)
        }
    }

    fn render(
        &self,
        module: &Module,
        _build_dir: &Utf8Path,
        rules: &IndexMap<String, &Rule>,
        env: &HashMap<&String, String>,
    ) -> Result<Vec<String>> {
        let mut rule_env = IndexMap::new();
        let rulename = match &self.source {
            Source::Git(Git::Commit { url, commit }) => {
                rule_env.insert("commit".to_string(), commit.to_string());
                rule_env.insert("url".to_string(), url.to_string());
                "GIT_DOWNLOAD"
            }
            _ => return Err(anyhow!("unsupported download type")),
        };

        let download_rule = match rules.values().find(|rule| rule.name == rulename) {
            Some(x) => x,
            None => panic!("missing {} rule for module {}", rulename, module.name),
        };

        let expanded = nested_env::expand_eval(&download_rule.cmd, env, IfMissing::Ignore)?;

        let ninja_download_rule = download_rule.to_ninja().command(expanded).build().unwrap();

        // "srcdir" is filled in data.rs
        let srcdir = module.srcdir.as_ref().unwrap();
        let tagfile = self.tagfile_download(srcdir);

        let ninja_download_build = NinjaBuildBuilder::default()
            .rule(&*ninja_download_rule.name)
            .out(tagfile.as_path())
            .env(&rule_env)
            .build()
            .unwrap();

        let mut ninja_snips = vec![
            ninja_download_rule.to_string(),
            ninja_download_build.to_string(),
        ];

        if self.patches.is_some() {
            ninja_snips.extend(self.patch(module, rules, env)?);
        }

        Ok(ninja_snips)
    }

    fn patch(
        &self,
        module: &Module,
        rules: &IndexMap<String, &Rule>,
        env: &HashMap<&String, String>,
    ) -> Result<Vec<String>> {
        let patches = self.patches.as_ref().unwrap();
        let rulename = match &self.source {
            Source::Git { .. } => "GIT_PATCH",
            _ => return Err(anyhow!("unsupported download type for patching")),
        };

        let patch_rule = match rules.values().find(|rule| rule.name == rulename) {
            Some(x) => x,
            None => panic!("missing {} rule for module {}", rulename, module.name),
        };

        let expanded = nested_env::expand_eval(&patch_rule.cmd, env, IfMissing::Ignore).unwrap();

        let ninja_patch_rule = patch_rule.to_ninja().command(expanded).build().unwrap();

        // "srcdir" is filled in data.rs
        let srcdir = module.srcdir.as_ref().unwrap();
        let tagfile_download = self.tagfile_download(srcdir);
        let tagfile_patched = self.tagfile_patched(srcdir);

        let patches = patches
            .iter()
            .map(|x| Cow::from(Utf8Path::new(module.relpath.as_ref().unwrap()).join(x)))
            .collect_vec();

        let download_dep = std::iter::once(&tagfile_download)
            .map(|x| Cow::from(x.as_ref()))
            .collect_vec();

        let ninja_patch_build = NinjaBuildBuilder::default()
            .rule(&*ninja_patch_rule.name)
            .inputs(patches)
            .out(tagfile_patched.as_path())
            .deps(Some(download_dep))
            .build()
            .unwrap();

        Ok(vec![
            ninja_patch_rule.to_string(),
            ninja_patch_build.to_string(),
        ])
    }
}

pub fn handle_module(
    module: &Module,
    build_dir: &Utf8Path,
    rules: &IndexMap<String, &Rule>,
    env: &HashMap<&String, String>,
) -> Result<Option<Vec<String>>> {
    if let Some(download) = &module.download {
        Ok(Some(download.render(module, build_dir, rules, env)?))
    } else {
        Ok(None)
    }
}
