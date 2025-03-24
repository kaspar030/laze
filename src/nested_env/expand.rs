/* this is based on "far" (https://forge.typ3.tech/charles/far) */

use evalexpr::EvalexprError;
use im::HashMap;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ExpandError {
    #[error("missing variable \"{0}\"")]
    Missing(String),
    #[error("unclosed brace at pos {0}")]
    Unclosed(usize),
    #[error("cycle involving variable \"{0}\"")]
    Cycle(String),
    #[error("expression error in `{1}`: {0}")]
    Expr(#[source] EvalexprError, String),
    #[error("while expanding `{1}`")]
    Nested(#[source] Box<ExpandError>, String),
}

#[derive(Debug, Copy, Clone)]
pub enum IfMissing {
    #[allow(dead_code)]
    Error,
    Ignore,
    Empty,
}

pub fn expand<SI, H>(
    f: SI,
    r: &HashMap<&String, String, H>,
    if_missing: IfMissing,
) -> Result<String, ExpandError>
where
    SI: AsRef<str>,
    H: std::hash::BuildHasher,
{
    let seen = Vec::new();
    expand_recursive::<SI, H>(f, r, seen, if_missing)
}

pub fn expand_eval<SI, H>(
    f: SI,
    r: &HashMap<&String, String, H>,
    if_missing: IfMissing,
) -> Result<String, ExpandError>
where
    SI: AsRef<str>,
    H: std::hash::BuildHasher,
{
    use crate::nested_env::Eval;
    let seen = Vec::new();
    let expanded = expand_recursive::<SI, H>(f, r, seen, if_missing)?;
    expanded.eval()
}

fn expand_recursive<'a, SI, H>(
    f: SI,
    r: &HashMap<&String, String, H>,
    seen: Vec<&'a str>,
    if_missing: IfMissing,
) -> Result<String, ExpandError>
where
    SI: 'a + AsRef<str>,
    H: std::hash::BuildHasher,
{
    let f = f.as_ref();

    // Stores (key, (key_start, key_end))
    let mut replaces = Vec::new();

    // Current position in the format string
    let mut cursor = 0;

    let mut escapes = false;
    while cursor < f.len() {
        if let Some(start) = (f[cursor..]).find("${") {
            if start > 0 && (&f[cursor..])[start - 1..start] == *"\\" {
                cursor += start + 1;
                escapes = true;
                continue;
            }
            let start = start + cursor;
            cursor = start;
            if let Some(end) = (f[cursor..]).find('}') {
                let end = end + cursor;
                replaces.push((
                    // The extracted key
                    &f[(start + "${".len())..end],
                    (
                        // Points to the `$` in the `${`
                        start,
                        // Just after the matching `}`
                        (end + "}".len()),
                    ),
                ));

                // Move cursor to the end of this match
                cursor = end + "}".len();
            } else {
                return Err(ExpandError::Unclosed(start));
            }
        } else {
            // No more matches
            break;
        }
    }

    /* TODO: return Cow String if there are no replaces */

    /* TODO: figure out useful value */
    let mut result = String::with_capacity(f.len() * 2);

    let mut cursor = 0;

    for (key, (start, end)) in replaces.into_iter() {
        /* TODO: there's a lot of cloning and string copying in this block.
         * Need to improve... */
        let mut seen = seen.clone();
        let key_ = &key.to_string();
        result.push_str(&f[cursor..start]);
        if seen.contains(&key) {
            return Err(ExpandError::Cycle(key.into()));
        }
        seen.push(key_);

        match r.get(key_) {
            Some(val) => result.push_str(expand_recursive(val, r, seen, if_missing)?.as_ref()),
            None => match if_missing {
                IfMissing::Error => return Err(ExpandError::Missing(key.into())),
                IfMissing::Ignore => {
                    result.push_str("${");
                    result.push_str(key_);
                    result.push('}')
                }
                IfMissing::Empty => (),
            },
        };
        cursor = end;
    }

    // If there's more text after the final `${}`
    if cursor < f.len() {
        result.push_str(&f[cursor..]);
    }

    if escapes {
        result = result.replace("\\${", "${");
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_expansion() {
        let vars = HashMap::new();
        assert_eq!(
            expand("simple string", &vars, IfMissing::Error),
            Ok("simple string".to_string())
        );
    }

    #[test]
    fn single_expansion() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "a".to_string());
        let vars: HashMap<&String, String> = vars.iter().map(|(k, v)| (k, v.into())).collect();
        assert_eq!(
            expand("${A} simple string", &vars, IfMissing::Error),
            Ok("a simple string".to_string())
        );
    }

    #[test]
    fn multi_expansion() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "a".to_string());
        vars.insert("B".to_string(), "with variables".to_string());
        let vars: HashMap<&String, String> = vars.iter().map(|(k, v)| (k, v.into())).collect();
        assert_eq!(
            expand("${A} simple string ${B}", &vars, IfMissing::Error),
            Ok("a simple string with variables".to_string())
        );
    }

    #[test]
    fn error_missing() {
        let vars = HashMap::new();
        assert_eq!(
            expand("simple string ${A}", &vars, IfMissing::Error),
            Err(ExpandError::Missing("A".to_string()))
        );
    }

    #[test]
    fn no_error_missing() {
        let vars = HashMap::new();
        assert_eq!(
            expand("simple string ${A}", &vars, IfMissing::Empty),
            Ok("simple string ".to_string())
        );
    }

    #[test]
    fn recursive() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "a(${B})".to_string());
        vars.insert("B".to_string(), "b()".to_string());
        let vars: HashMap<&String, String> = vars.iter().map(|(k, v)| (k, v.into())).collect();
        assert_eq!(
            expand("x${A}x", &vars, IfMissing::Error),
            Ok("xa(b())x".to_string())
        );
    }

    #[test]
    fn single_expansion_escaped() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "\\${a}".to_string());
        let vars: HashMap<&String, String> = vars.iter().map(|(k, v)| (k, v.into())).collect();
        assert_eq!(
            expand("${A} simple string", &vars, IfMissing::Error),
            Ok("${a} simple string".to_string())
        );
    }
}
