use std::collections::HashMap;
use std::vec::Vec;

mod expand;

pub use expand::{expand, IfMissing};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvKey {
    Single(String),
    List(Vec<String>),
}

impl EnvKey {
    fn merge(&self, other: &EnvKey) -> EnvKey {
        match self {
            EnvKey::Single(_) => other.clone(),
            EnvKey::List(self_values) => match other {
                EnvKey::Single(_) => other.clone(),
                EnvKey::List(other_values) => EnvKey::List(
                    self_values
                        .iter()
                        .chain(other_values)
                        .map(|x| x.clone())
                        .collect(),
                ),
            },
        }
    }

    fn flatten(&self) -> String {
        match self {
            EnvKey::Single(s) => s.clone(),
            EnvKey::List(list) => list.join(" "),
        }
    }
}

pub type Env = HashMap<String, EnvKey>;

pub fn merge(lower: &mut Env, upper: &Env) {
    for (upper_key, upper_value) in upper {
        lower
            .entry(upper_key.clone())
            .and_modify(|e| *e = e.merge(upper_value))
            .or_insert(upper_value.clone());
    }
}

pub fn flatten(env: &Env) -> HashMap<&String, String> {
    env.iter()
        .map(|(key, value)| (key, value.flatten()))
        .collect()
}

pub fn flatten_expand<'a>(flattened: &'a HashMap<&String, String>) -> HashMap<&'a String, String> {
    flattened
        .iter()
        .map(|(key, value)| (*key, expand(value, flattened, IfMissing::Error).unwrap()))
        .collect()
}

pub fn expand_env(env: &Env, values: &Env) -> Env {
    let values = flatten(values);

    fn expand_envkey(envkey: &EnvKey, values: &HashMap<&String, String>) -> EnvKey {
        match envkey {
            EnvKey::Single(key) => EnvKey::Single(expand(key, values, IfMissing::Ignore).unwrap()),
            EnvKey::List(keys) => EnvKey::List({
                keys.iter()
                    .map(|x| expand(x, values, IfMissing::Ignore).unwrap())
                    .collect()
            }),
        }
    }

    let mut result = Env::with_capacity(env.len());
    for (key, value) in env {
        result.insert(key.clone(), expand_envkey(value, &values));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_nonexisting_single() {
        let mut upper = Env::new();
        let lower = Env::new();
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );

        let mut merged = Env::new();
        merge(&mut merged, &lower);
        merge(&mut merged, &upper);
        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::Single("upper_value".to_string())
        );
    }

    #[test]
    fn test_merge_overwriting_single() {
        let mut upper = Env::new();
        let mut lower = Env::new();
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );

        lower.insert(
            "mykey".to_string(),
            EnvKey::Single("lower_value".to_string()),
        );

        let mut merged = Env::new();
        merge(&mut merged, &lower);
        merge(&mut merged, &upper);
        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::Single("upper_value".to_string())
        );
    }

    #[test]
    fn test_merge_overwriting_list() {
        let mut upper = Env::new();
        let mut lower = Env::new();
        lower.insert(
            "mykey".to_string(),
            EnvKey::List(vec![
                "lower_value_1".to_string(),
                "lower_value_2".to_string(),
            ]),
        );
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );

        let mut merged = Env::new();
        merge(&mut merged, &lower);
        merge(&mut merged, &upper);
        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::Single("upper_value".to_string())
        );
    }

    #[test]
    fn test_merge_overwriting_with_list() {
        let mut upper = Env::new();
        let mut lower = Env::new();
        lower.insert(
            "mykey".to_string(),
            EnvKey::Single("lower_value".to_string()),
        );

        upper.insert(
            "mykey".to_string(),
            EnvKey::List(vec![
                "upper_value_1".to_string(),
                "upper_value_2".to_string(),
            ]),
        );

        let mut merged = Env::new();
        merge(&mut merged, &lower);
        merge(&mut merged, &upper);
        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::List(vec![
                "upper_value_1".to_string(),
                "upper_value_2".to_string(),
            ]),
        );
    }

    #[test]
    fn test_merge_merging_list() {
        let mut upper = Env::new();
        let mut lower = Env::new();
        lower.insert(
            "mykey".to_string(),
            EnvKey::List(vec![
                "lower_value_1".to_string(),
                "lower_value_2".to_string(),
            ]),
        );

        upper.insert(
            "mykey".to_string(),
            EnvKey::List(vec![
                "upper_value_1".to_string(),
                "upper_value_2".to_string(),
            ]),
        );

        let mut merged = Env::new();
        merge(&mut merged, &lower);
        merge(&mut merged, &upper);
        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::List(vec![
                "lower_value_1".to_string(),
                "lower_value_2".to_string(),
                "upper_value_1".to_string(),
                "upper_value_2".to_string(),
            ]),
        );
    }

    #[test]
    fn test_basic() {
        let mut upper = Env::new();
        let mut lower = Env::new();
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );
        lower.insert(
            "mykey".to_string(),
            EnvKey::Single("lower_value".to_string()),
        );

        let merged = Env::new();
        for (key, value) in merged {
            dbg!(key, value);
        }
    }
}
