use std::collections::HashMap;
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
    #[serde(default = "default_as_false")]
    pub always: bool,
}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /* rules are unique per context subtree, so hashing the name is
         * sufficient. */
        self.name.hash(state);
    }
}
