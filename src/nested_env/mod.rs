use anyhow::{Context, Error};
use evalexpr::EvalexprError;
use im::{hashmap::Entry, vector, HashMap, Vector};
use itertools::join;

mod expand;
mod expr;
pub use expr::Eval;

pub use expand::{expand, expand_eval, IfMissing};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct Env {
    #[serde(flatten)]
    inner: HashMap<String, EnvKey>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[serde(untagged, expecting = "expected single value or array of values")]
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

    fn flatten(&self) -> Result<String, EvalexprError> {
        match self {
            EnvKey::Single(s) => Ok(s.clone()),
            EnvKey::List(list) => Ok(join(list, " ")),
        }
    }

    fn flatten_with_opts(&self, opts: &MergeOption) -> Result<String, EvalexprError> {
        let mut res = String::new();
        if let Some(start) = &opts.start {
            res.push_str(start);
        }

        match self {
            EnvKey::Single(s) => {
                if let Some(prefix) = &opts.prefix {
                    res.push_str(prefix);
                }

                res.push_str(s);

                if let Some(suffix) = &opts.suffix {
                    res.push_str(suffix);
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
                        res.push_str(prefix);
                    }

                    res.push_str(s);

                    if let Some(suffix) = &opts.suffix {
                        res.push_str(suffix);
                    }
                    if pos != last {
                        res.push_str(joiner);
                    }
                }
            }
        }
        if let Some(end) = &opts.end {
            res.push_str(&end[..]);
        }
        Ok(res)
    }
}

impl Env {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: &Env) {
        for (key, value) in other.inner.iter() {
            match self.entry(key.clone()) {
                Entry::Vacant(e) => {
                    e.insert(value.clone());
                }
                Entry::Occupied(mut e) => {
                    let merged = e.get_mut().merge(&value);
                    *e.get_mut() = merged;
                }
            }
        }
    }

    pub fn flatten(&self) -> Result<HashMap<&String, String>, Error> {
        self.inner
            .iter()
            .map(|(key, value)| {
                match value.flatten() {
                    Ok(v) => Ok((key, v)),
                    Err(e) => Err(e),
                }
                .with_context(|| format!("variable \"{key}\""))
            })
            .collect::<Result<HashMap<_, _>, Error>>()
    }
    pub fn flatten_with_opts<'a>(
        &'a self,
        merge_opts: &HashMap<String, MergeOption>,
    ) -> Result<HashMap<&'a String, String>, Error> {
        self.inner
            .iter()
            .map(|(key, value)| {
                if let Some(merge_opts) = merge_opts.get(key) {
                    match value.flatten_with_opts(merge_opts) {
                        Ok(v) => Ok((key, v)),
                        Err(e) => Err(e),
                    }
                } else {
                    match value.flatten() {
                        Ok(v) => Ok((key, v)),
                        Err(e) => Err(e),
                    }
                }
                .with_context(|| format!("variable \"{key}\""))
            })
            .collect::<Result<_, _>>()
    }

    pub fn flatten_with_opts_option<'a>(
        &'a self,
        merge_opts: Option<&HashMap<String, MergeOption>>,
    ) -> Result<HashMap<&'a String, String>, Error> {
        if let Some(merge_opts) = merge_opts {
            self.flatten_with_opts(merge_opts)
        } else {
            self.flatten()
        }
    }

    // pub fn flatten_expand<'a>(flattened: &'a HashMap<&String, String>) -> HashMap<&'a String, String> {
    //     flattened
    //         .iter()
    //         .map(|(key, value)| (*key, expand(value, flattened, IfMissing::Error).unwrap()))
    //         .collect()
    // }

    pub fn expand(&mut self, values: &Env) -> Result<(), Error> {
        let values = values.flatten()?;

        fn expand_envkey(envkey: &EnvKey, values: &HashMap<&String, String>) -> EnvKey {
            match envkey {
                EnvKey::Single(key) => {
                    EnvKey::Single(expand(key, values, IfMissing::Ignore).unwrap())
                }
                EnvKey::List(keys) => EnvKey::List({
                    keys.iter()
                        .map(|x| expand(x, values, IfMissing::Ignore).unwrap())
                        .collect()
                }),
            }
        }

        for (_, value) in self.inner.iter_mut() {
            *value = expand_envkey(value, &values);
        }

        Ok(())
    }

    pub fn insert(&mut self, key: String, value: EnvKey) -> Option<EnvKey> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<&EnvKey> {
        self.inner.get(key)
    }

    pub fn entry(
        &mut self,
        key: String,
    ) -> im::hashmap::Entry<'_, String, EnvKey, std::collections::hash_map::RandomState> {
        self.inner.entry(key)
    }

    pub fn assign_from_string(&mut self, assignment: &str) -> Result<(), anyhow::Error> {
        if let Some((var, value)) = assignment.split_once("+=") {
            let mut new = Env::new();
            new.insert(var.to_string(), EnvKey::List(vector![value.to_owned()]));
            self.merge(&new);
        } else if let Some((var, value)) = assignment.split_once('=') {
            let mut new = Env::new();
            new.insert(var.to_string(), EnvKey::Single(value.to_string()));
            self.merge(&new);
        } else {
            return Err(anyhow!(format!(
                "cannot parse assignment from \"{}\"",
                assignment
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use im::vector;

    #[test]
    fn test_merge_nonexisting_single() {
        let mut upper = Env::new();
        let mut lower = Env::new();
        upper.insert(
            "mykey".to_string(),
            EnvKey::Single("upper_value".to_string()),
        );

        lower.merge(&upper);

        let merged = lower;

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

        lower.merge(&upper);

        assert_eq!(
            lower.get("mykey").unwrap(),
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

        lower.merge(&upper);

        assert_eq!(
            lower.get("mykey").unwrap(),
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

        lower.merge(&upper);

        assert_eq!(
            lower.get("mykey").unwrap(),
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

        lower.merge(&upper);

        assert_eq!(
            lower.get("mykey").unwrap(),
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

        lower.merge(&upper);
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

        let flattened = env.flatten_with_opts(&merge_opts).unwrap();

        assert_eq!(
            flattened.get(&"mykey".to_string()).unwrap(),
            &"(Pvalue_1S,Pvalue_2S,Pvalue_3S,Pvalue_4S)".to_string()
        );
    }

    #[test]
    fn test_assign_from_string_override() {
        let mut env = Env::new();
        env.insert("FOO".to_string(), EnvKey::Single("whiskeyBAR".to_string()));

        env.assign_from_string("FOO=milkBAR").unwrap();

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

        env.assign_from_string("FOO=milkBAR").unwrap();

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

        env.assign_from_string("FOO+=milkBAR").unwrap();

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
