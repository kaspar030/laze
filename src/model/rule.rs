use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::nested_env::EnvMap;
use crate::serde_bool_helpers::{default_as_false, default_as_true};

#[derive(Debug, Serialize, Deserialize, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub name: String,
    pub cmd: String,

    pub help: Option<String>,

    #[serde(rename = "in")]
    pub in_: Option<String>,
    pub out: Option<String>,
    pub context: Option<String>,
    pub options: Option<HashMap<String, String>>,
    pub gcc_deps: Option<String>,
    pub rspfile: Option<String>,
    pub rspfile_content: Option<String>,
    pub pool: Option<String>,
    pub description: Option<String>,
    pub export: Option<Vec<VarExportSpec>>,

    #[serde(default = "default_as_false")]
    pub always: bool,
    #[serde(default = "default_as_true")]
    pub shareable: bool,
}

impl Rule {
    pub fn to_ninja(&self, env: &EnvMap) -> anyhow::Result<NinjaRule> {
        let ninja_rule: NinjaRuleBuilder = self.into();
        Ok(ninja_rule.build().unwrap().expand(env)?.named())
    }

    /// get rule description
    ///
    /// if no description is set, uses this rule's name
    pub fn description(&self) -> Cow<str> {
        if let Some(description) = &self.description {
            Cow::from(description)
        } else {
            Cow::from(&self.name)
        }
    }
}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /* rules are unique per context subtree, so hashing the name is
         * sufficient. */
        self.name.hash(state);
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

use crate::ninja::{NinjaRule, NinjaRuleBuilder};

use super::VarExportSpec;

impl<'a> From<&'a Rule> for crate::ninja::NinjaRuleBuilder<'a> {
    fn from(rule: &'a Rule) -> Self {
        let mut builder = NinjaRuleBuilder::default();
        builder
            .name(Cow::from(&rule.name))
            .description(Some(rule.description()))
            .command(&rule.cmd)
            .rspfile(rule.rspfile.as_deref().map(Cow::from))
            .rspfile_content(rule.rspfile_content.as_deref().map(Cow::from))
            .pool(rule.pool.as_deref().map(Cow::from))
            .always(rule.always)
            .export(&rule.export)
            .deps(rule.gcc_deps.as_ref());

        builder
    }
}
