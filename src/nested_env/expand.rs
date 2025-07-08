/* this is based on "far" (https://forge.typ3.tech/charles/far) */
use std::error;
use std::fmt;

use evalexpr::EvalexprError;

use super::EnvMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ExpandError {
    Missing(String),
    Unclosed(usize),
    Cycle(String),
    Expr(EvalexprError),
}

#[derive(Debug, Copy, Clone)]
pub enum IfMissing {
    #[allow(dead_code)]
    Error,
    Ignore,
    Empty,
}

impl fmt::Display for ExpandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExpandError::Missing(s) => write!(f, "missing variable \"{}\"", s),
            ExpandError::Cycle(s) => write!(f, "cycle involving variable \"{}\"", s),
            ExpandError::Unclosed(start) => write!(f, "unclosed brace at pos {}", start),
            ExpandError::Expr(e) => write!(f, "expression error: {}", e),
        }
    }
}

impl error::Error for ExpandError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

pub fn expand<SI>(f: SI, r: &EnvMap, if_missing: IfMissing) -> Result<String, ExpandError>
where
    SI: AsRef<str>,
{
    let seen = Vec::new();
    expand_recursive::<SI>(f, r, seen, if_missing)
}

pub fn expand_eval<SI>(f: SI, r: &EnvMap, if_missing: IfMissing) -> Result<String, ExpandError>
where
    SI: AsRef<str>,
{
    use crate::nested_env::Eval;
    let seen = Vec::new();
    Ok(expand_recursive::<SI>(f, r, seen, if_missing)?
        .eval()
        .map_err(ExpandError::Expr))?
}

fn expand_recursive<'a, SI>(
    f: SI,
    r: &EnvMap,
    seen: Vec<&'a str>,
    if_missing: IfMissing,
) -> Result<String, ExpandError>
where
    SI: 'a + AsRef<str>,
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
        result.push_str(&f[cursor..start]);
        if seen.contains(&key) {
            return Err(ExpandError::Cycle(key.into()));
        }
        seen.push(key);

        match r.get(key) {
            Some(val) => result.push_str(expand_recursive(val, r, seen, if_missing)?.as_ref()),
            None => match if_missing {
                IfMissing::Error => return Err(ExpandError::Missing(key.into())),
                IfMissing::Ignore => {
                    result.push_str("${");
                    result.push_str(key);
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
        let vars = EnvMap::new();
        assert_eq!(
            expand("simple string", &vars, IfMissing::Error),
            Ok("simple string".to_string())
        );
    }

    #[test]
    fn single_expansion() {
        let mut vars = EnvMap::new();
        vars.insert("A", "a".into());
        assert_eq!(
            expand("${A} simple string", &vars, IfMissing::Error),
            Ok("a simple string".to_string())
        );
    }

    #[test]
    fn multi_expansion() {
        let mut vars = EnvMap::new();
        vars.insert("A", "a".into());
        vars.insert("B", "with variables".into());
        assert_eq!(
            expand("${A} simple string ${B}", &vars, IfMissing::Error),
            Ok("a simple string with variables".to_string())
        );
    }

    #[test]
    fn error_missing() {
        let vars = EnvMap::new();
        assert_eq!(
            expand("simple string ${A}", &vars, IfMissing::Error),
            Err(ExpandError::Missing("A".to_string()))
        );
    }

    #[test]
    fn no_error_missing() {
        let vars = EnvMap::new();
        assert_eq!(
            expand("simple string ${A}", &vars, IfMissing::Empty),
            Ok("simple string ".to_string())
        );
    }

    #[test]
    fn recursive() {
        let mut vars = EnvMap::new();
        vars.insert("A", "a(${B})".into());
        vars.insert("B", "b()".into());
        assert_eq!(
            expand("x${A}x", &vars, IfMissing::Error),
            Ok("xa(b())x".to_string())
        );
    }

    #[test]
    fn single_expansion_escaped() {
        let mut vars = EnvMap::new();
        vars.insert("A", "\\${a}".into());
        assert_eq!(
            expand("${A} simple string", &vars, IfMissing::Error),
            Ok("${a} simple string".to_string())
        );
    }
}
