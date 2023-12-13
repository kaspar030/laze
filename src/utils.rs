use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StringOrMapString {
    String(String),
    Map(std::collections::HashMap<String, Vec<String>>),
}
