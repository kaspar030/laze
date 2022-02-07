use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::hash::{Hash, Hasher};

use crate::serde_bool_helpers::default_as_false;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Rule {
    pub name: String,
    pub cmd: String,

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

    /* make this rule's command show up in compile_commands.json */
    #[serde(default = "default_as_false")]
    pub compile_command: bool,

    #[serde(default = "default_as_false")]
    pub always: bool,
}

impl Rule {
    pub fn to_ninja(&self) -> NinjaRuleBuilder {
        self.into()
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

use crate::ninja::{NinjaRuleBuilder, NinjaRuleDeps};

impl<'a> From<&'a Rule> for crate::ninja::NinjaRuleBuilder<'a> {
    fn from(rule: &'a Rule) -> Self {
        let mut builder = NinjaRuleBuilder::default();
        builder
            .name(Cow::from(&rule.name))
            .description(Some(rule.description()))
            .rspfile(rule.rspfile.as_deref().map(Cow::from))
            .rspfile_content(rule.rspfile_content.as_deref().map(Cow::from))
            .pool(rule.pool.as_deref().map(Cow::from))
            .deps(match &rule.gcc_deps {
                None => NinjaRuleDeps::None,
                Some(s) => NinjaRuleDeps::GCC(s.into()),
            });
        builder
    }
}
