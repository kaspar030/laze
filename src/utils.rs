use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use camino::Utf8PathBuf;
use indexmap::IndexMap;

pub(crate) fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum StringOrMapVecString {
    String(String),
    Map(std::collections::HashMap<String, Vec<String>>),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum StringOrMapString {
    String(String),
    Map(std::collections::HashMap<String, String>),
}

pub(crate) trait ContainingPath<T: AsRef<std::path::Path>> {
    fn get_containing_path(&self, path: &T) -> Option<&T>;
}

impl ContainingPath<Utf8PathBuf> for IndexMap<&Utf8PathBuf, Utf8PathBuf> {
    fn get_containing_path(&self, path: &Utf8PathBuf) -> Option<&Utf8PathBuf> {
        self.get(path).or_else(|| {
            self.iter()
                .find(|(k, _)| path.starts_with(k))
                .map(|(_, v)| v)
        })
    }
}
