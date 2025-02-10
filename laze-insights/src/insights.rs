use camino::Utf8PathBuf;
use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Insights {
    pub builds: IndexMap<String, IndexMap<String, InsightBuildInfo>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InsightBuildInfo {
    pub outfile: Utf8PathBuf,
    pub modules: IndexMap<String, ModuleInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModuleInfo {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub deps: Vec<String>,
}
