use camino::Utf8PathBuf;
use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Insights {
    builds: IndexMap<String, IndexMap<String, InsightBuildInfo>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InsightBuildInfo {
    pub outfile: Utf8PathBuf,
    pub modules: IndexMap<String, ModuleInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModuleInfo {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
}

#[cfg(not(feature = "building-laze-insights"))]
mod not_for_lib {
    use super::{InsightBuildInfo, Insights};
    use crate::generate::BuildInfo;

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

    impl From<&BuildInfo> for InsightBuildInfo {
        fn from(build_info: &BuildInfo) -> Self {
            Self {
                outfile: build_info.out.clone(),
                modules: build_info.module_info.as_ref().unwrap().clone(),
            }
        }
    }
}
