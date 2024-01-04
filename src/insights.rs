use camino::Utf8PathBuf;
use indexmap::IndexMap;

use crate::generate::{BuildInfo, ModuleInfo};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Insights {
    builds: IndexMap<String, IndexMap<String, InsightBuildInfo>>,
}

impl Insights {
    pub fn from_builds(builds: &Vec<BuildInfo>) -> Insights {
        let mut insights = Insights::default();
        for build_info in builds {
            insights
                .builds
                .entry(build_info.builder.clone())
                .or_default()
                .insert(
                    build_info.binary.clone(),
                    InsightBuildInfo::from(build_info),
                );
        }
        insights
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InsightBuildInfo {
    pub outfile: Utf8PathBuf,
    pub modules: IndexMap<String, ModuleInfo>,
}

impl From<&BuildInfo> for InsightBuildInfo {
    fn from(build_info: &BuildInfo) -> Self {
        Self {
            outfile: build_info.out.clone(),
            modules: build_info.module_info.as_ref().unwrap().clone(),
        }
    }
}
