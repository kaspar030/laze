//! This module deals with "download:" directives

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{
    ninja::NinjaBuildBuilder, ninja::NinjaRuleBuilder, util::path_clone_push, Module, Rule,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum DownloadSource {
    #[serde(rename = "git")]
    Git { url: String, commit: String },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Download {
    #[serde(flatten)]
    pub source: DownloadSource,
    pub patches: Option<Vec<String>>,
}

impl Download {
    pub fn srcdir(&self, build_dir: &Path, module: &Module) -> PathBuf {
        let mut srcdir = PathBuf::from(build_dir);
        srcdir.push("dl");
        srcdir.push(module.relpath.as_ref().unwrap().clone());
        srcdir.push(module.name.clone());
        srcdir
    }

    pub fn tagfile(&self, srcdir: &PathBuf) -> PathBuf {
        path_clone_push(&srcdir, ".laze-downloaded")
    }

    fn render(
        &self,
        module: &Module,
        _build_dir: &Path,
        rules: &IndexMap<String, &Rule>,
    ) -> Result<Vec<String>> {
        let mut env = HashMap::new();
        let rulename = match &self.source {
            DownloadSource::Git { commit, url } => {
                env.insert("commit".to_string(), commit.to_string());
                env.insert("url".to_string(), url.to_string());
                "GIT_DOWNLOAD"
            }
        };

        let download_rule = match rules.values().find(|rule| rule.name == rulename) {
            Some(x) => x,
            None => panic!("missing {} rule for module {}", rulename, module.name),
        };

        // TODO: expand with global env?
        let expanded = &download_rule.cmd;

        let ninja_download_rule = NinjaRuleBuilder::default()
            .name(&download_rule.name)
            .description(Some(Cow::from(&download_rule.name)))
            .command(expanded)
            .rspfile(download_rule.rspfile.as_deref().map(Cow::from))
            .rspfile_content(download_rule.rspfile_content.as_deref().map(Cow::from))
            .build()
            .unwrap();

        // "srcdir" is filled in data.rs
        let srcdir = module.srcdir.as_ref().unwrap();
        let tagfile = self.tagfile(srcdir);

        let ninja_download_build = NinjaBuildBuilder::default()
            .rule(&*ninja_download_rule.name)
            .out(tagfile.as_path())
            .env(&env)
            .build()
            .unwrap();

        Ok(vec![
            ninja_download_rule.to_string(),
            ninja_download_build.to_string(),
        ])
    }
}

pub fn handle_module(
    module: &Module,
    build_dir: &Path,
    rules: &IndexMap<String, &Rule>,
) -> Result<Option<Vec<String>>> {
    if let Some(download) = &module.download {
        Ok(Some(download.render(module, build_dir, rules)?))
    } else {
        Ok(None)
    }
}
