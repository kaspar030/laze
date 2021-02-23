//! This module deals with "download:" directives
use serde::{Deserialize, Serialize};

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
