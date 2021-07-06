use im::vector;
use im::HashMap;
use im::Vector;
use itertools::join;

mod expand;

pub use expand::{expand, IfMissing};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[serde(untagged)]
pub enum EnvKey {
    Single(String),
    List(Vector<String>),
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
                EnvKey::List(other_values) => {
                    let mut combined = self_values.clone();
                    combined.append(other_values.clone());
                    EnvKey::List(combined)
                }
            },
        }
    }

    fn flatten(&self) -> String {
        match self {
            EnvKey::Single(s) => s.clone(),
            EnvKey::List(list) => join(list, " "),
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

pub fn merge(lower: Env, upper: Env) -> Env {
    lower.union_with(upper, |lower_value, upper_value| {
        lower_value.merge(&upper_value)
    })
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

    env.iter()
        .map(|(key, val)| (key.clone(), expand_envkey(val, &values)))
        .collect()
}

pub fn assign_from_string(env: Env, assignment: &str) -> Result<Env, anyhow::Error> {
    let res;
    if let Some((var, value)) = assignment.split_once("+=") {
        let mut new = Env::new();
        new.insert(var.to_string(), EnvKey::List(vector![value.to_owned()]));
        res = merge(env, new);
    } else if let Some((var, value)) = assignment.split_once("=") {
        let mut new = Env::new();
        new.insert(var.to_string(), EnvKey::Single(value.to_string()));
        res = merge(env, new);
    } else {
        return Err(anyhow!(format!(
            "cannot parse assignment from \"{}\"",
            assignment
        )));
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use im::vector;

    #[test]
    fn test_merge_nonexisting_single() {
        let mut upper = Env::new();
        let lower = Env::new();
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );

        let merged = merge(lower, upper);

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

        let merged = merge(lower, upper);

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
            EnvKey::List(vector![
                "lower_value_1".to_string(),
                "lower_value_2".to_string(),
            ]),
        );
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );

        let merged = merge(lower, upper);

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
            EnvKey::List(vector![
                "upper_value_1".to_string(),
                "upper_value_2".to_string(),
            ]),
        );

        let merged = merge(lower, upper);

        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::List(vector![
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
            EnvKey::List(vector![
                "lower_value_1".to_string(),
                "lower_value_2".to_string(),
            ]),
        );

        upper.insert(
            "mykey".to_string(),
            EnvKey::List(vector![
                "upper_value_1".to_string(),
                "upper_value_2".to_string(),
            ]),
        );

        let merged = merge(lower, upper);

        assert_eq!(
            merged.get("mykey").unwrap(),
            &EnvKey::List(vector![
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

        let merged = merge(lower, upper);

        for (key, value) in merged {
            dbg!(key, value);
        }
    }

    #[test]
    fn test_mergeopts() {
        let mut env = Env::new();
        env.insert(
            "mykey".to_string(),
            EnvKey::List(vector![
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

    #[test]
    fn test_assign_from_string_override() {
        let mut env = Env::new();
        env.insert("FOO".to_string(), EnvKey::Single("whiskeyBAR".to_string()));

        let env = assign_from_string(env, "FOO=milkBAR").unwrap();

        assert_eq!(
            env.get(&"FOO".to_string()).unwrap(),
            &EnvKey::Single("milkBAR".to_string()),
        );
    }

    #[test]
    fn test_assign_from_string_override_list() {
        let mut env = Env::new();
        env.insert(
            "FOO".to_string(),
            EnvKey::List(vector!["whiskeyBAR".to_string(), "beerBAR".to_string()]),
        );

        let env = assign_from_string(env, "FOO=milkBAR").unwrap();

        assert_eq!(
            env.get(&"FOO".to_string()).unwrap(),
            &EnvKey::Single("milkBAR".to_string()),
        );
    }

    #[test]
    fn test_assign_from_string_append() {
        let mut env = Env::new();
        env.insert(
            "FOO".to_string(),
            EnvKey::List(vector!["whiskeyBAR".to_string(), "beerBAR".to_string()]),
        );

        let env = assign_from_string(env, "FOO+=milkBAR").unwrap();

        assert_eq!(
            env.get(&"FOO".to_string()).unwrap(),
            &EnvKey::List(vector![
                "whiskeyBAR".to_string(),
                "beerBAR".to_string(),
                "milkBAR".to_string()
            ]),
        );
    }
}
