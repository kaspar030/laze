//! This module deals with "download:" directives

use std::borrow::Cow;

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{ninja::NinjaBuildBuilder, ninja::NinjaRuleBuilder, Module, Rule};

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
    fn render(
        &self,
        module: &Module,
        build_dir: &Path,
        rules: &IndexMap<String, &Rule>,
    ) -> Result<(PathBuf, PathBuf, Vec<String>)> {
        let rulename = match self.source {
            DownloadSource::Git { .. } => "GIT_DOWNLOAD",
        };

        let download_rule = match rules.values().find(|rule| rule.name == rulename) {
            Some(x) => x,
            None => panic!("missing {} rule for module {}", rulename, module.name),
        };

        // TODO: expand with global env?
        // let expanded =
        //     nested_env::expand(&download_rule.cmd, &global_env_flattened, IfMissing::Empty)
        //         .unwrap();
        let expanded = &download_rule.cmd;

        let ninja_download_rule = NinjaRuleBuilder::default()
            .name(&download_rule.name)
            .description(Some(Cow::from(&download_rule.name)))
            .command(expanded)
            .rspfile(download_rule.rspfile.as_deref().map(Cow::from))
            .rspfile_content(download_rule.rspfile_content.as_deref().map(Cow::from))
            .build()
            .unwrap()
            .named();

        let mut dldir = PathBuf::from(build_dir);
        dldir.push("dl");
        dldir.push(module.relpath.as_ref().unwrap().clone());
        dldir.push(module.name.clone());
        let mut tagfile = dldir.clone();
        tagfile.push(".laze-downloaded");

        let ninja_download_build = NinjaBuildBuilder::default()
            .rule(&*ninja_download_rule.name)
            .out(tagfile.as_path())
            .build()
            .unwrap();

        Ok((
            dldir,
            tagfile.clone(),
            vec![
                ninja_download_rule.to_string(),
                ninja_download_build.to_string(),
            ],
        ))
    }
}

pub fn handle_module(
    module: &Module,
    build_dir: &Path,
    rules: &IndexMap<String, &Rule>,
) -> Result<Option<(PathBuf, PathBuf, Vec<String>)>> {
    if let Some(download) = &module.download {
        Ok(Some(download.render(module, build_dir, rules)?))
    } else {
        Ok(None)
    }
}
