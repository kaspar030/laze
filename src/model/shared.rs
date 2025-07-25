use serde::{Deserialize, Serialize};

use crate::nested_env::{self, EnvMap};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct VarExportSpec {
    pub variable: String,
    pub content: Option<String>,
}

impl VarExportSpec {
    pub fn apply_env(&self, env: &EnvMap) -> Self {
        let content = if let Some(content) = self.content.as_ref() {
            content.clone()
        } else {
            format!("${{{}}}", self.variable)
        };

        let content =
            Some(nested_env::expand_eval(content, env, nested_env::IfMissing::Empty).unwrap());

        Self {
            variable: self.variable.clone(),
            content,
        }
    }

    pub(crate) fn expand<
        'a,
        T: Iterator<Item = &'a VarExportSpec>,
        U: FromIterator<VarExportSpec>,
    >(
        export: T,
        env: &EnvMap,
    ) -> U {
        // what this does is, apply the env to the format as given by "export:"
        //
        // e.g., assuming `FOO=value` and FOOBAR=`other_value`:
        // ```yaml
        //
        // export:
        //   - FOO
        //   - BAR: bar
        //   - FOOBAR: ${foobar}
        // ```
        //
        // ... to export `FOO=value`, `BAR=bar` and `FOOBAR=other_value`.

        export.map(|entry| entry.apply_env(env)).collect()
    }
}
