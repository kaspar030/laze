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

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct MergeOption {
    joiner: Option<String>,
    prefix: Option<String>,
    suffix: Option<String>,
    start: Option<String>,
    end: Option<String>,
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

    fn flatten_with_opts(&self, opts: &MergeOption) -> String {
        let mut res = String::new();
        if let Some(start) = &opts.start {
            res.push_str(&start);
        }

        match self {
            EnvKey::Single(s) => {
                if let Some(prefix) = &opts.prefix {
                    res.push_str(&prefix);
                }

                res.push_str(&s);

                if let Some(suffix) = &opts.suffix {
                    res.push_str(&suffix);
                }
            }
            EnvKey::List(list) => {
                let joiner = match &opts.joiner {
                    Some(joiner) => joiner,
                    None => " ",
                };
                let last = list.len() - 1;
                for (pos, s) in list.iter().enumerate() {
                    if s.is_empty() {
                        continue;
                    }
                    if let Some(prefix) = &opts.prefix {
                        res.push_str(&prefix);
                    }

                    res.push_str(&s);

                    if let Some(suffix) = &opts.suffix {
                        res.push_str(&suffix);
                    }
                    if pos != last {
                        res.push_str(&joiner);
                    }
                }
            }
        }
        if let Some(end) = &opts.end {
            res.push_str(&end[..]);
        }
        res
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

pub fn flatten_with_opts<'a>(
    env: &'a Env,
    merge_opts: &HashMap<String, MergeOption>,
) -> HashMap<&'a String, String> {
    env.iter()
        .map(|(key, value)| {
            (
                key,
                if let Some(merge_opts) = merge_opts.get(key) {
                    value.flatten_with_opts(merge_opts)
                } else {
                    value.flatten()
                },
            )
        })
        .collect()
}

pub fn flatten_with_opts_option<'a>(
    env: &'a Env,
    merge_opts: Option<&HashMap<String, MergeOption>>,
) -> HashMap<&'a String, String> {
    if let Some(merge_opts) = merge_opts {
        flatten_with_opts(env, merge_opts)
    } else {
        flatten(env)
    }
}

// pub fn flatten_expand<'a>(flattened: &'a HashMap<&String, String>) -> HashMap<&'a String, String> {
//     flattened
//         .iter()
//         .map(|(key, value)| (*key, expand(value, flattened, IfMissing::Error).unwrap()))
//         .collect()
// }

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

    #[test]
    fn test_mergeopts() {
        let mut env = Env::new();
        env.insert(
            "mykey".to_string(),
            EnvKey::List(vec![
                "value_1".to_string(),
                "value_2".to_string(),
                "value_3".to_string(),
                "value_4".to_string(),
            ]),
        );

        let mut merge_opts = HashMap::new();
        merge_opts.insert(
            "mykey".to_string(),
            MergeOption {
                joiner: Some(",".to_string()),
                prefix: Some("P".to_string()),
                suffix: Some("S".to_string()),
                start: Some("(".to_string()),
                end: Some(")".to_string()),
            },
        );

        let flattened = flatten_with_opts(&env, &merge_opts);

        assert_eq!(
            flattened.get(&"mykey".to_string()).unwrap(),
            &"(Pvalue_1S,Pvalue_2S,Pvalue_3S,Pvalue_4S)".to_string()
        );
    }
}
