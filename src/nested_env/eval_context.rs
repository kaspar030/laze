use camino::Utf8PathBuf;
use evalexpr::{EvalexprError, EvalexprResult, Value};

use super::EnvMap;

pub struct EvalContext<'a, 'b: 'a> {
    inner: &'a EnvMap<'b>,
    values: bumpalo::Bump,
}

impl<'a, 'b: 'a> EvalContext<'a, 'b> {
    pub fn new(env: &'a EnvMap<'b>) -> Self {
        Self {
            inner: env,
            values: bumpalo::Bump::new(),
        }
    }
}

impl evalexpr::Context for EvalContext<'_, '_> {
    fn get_value(&self, identifier: &str) -> Option<&Value> {
        self.inner
            .get(identifier)
            .map(|s| &*self.values.alloc(Value::String(s.to_string())))
    }

    fn call_function(
        &self,
        identifier: &str,
        argument: &evalexpr::Value,
    ) -> evalexpr::EvalexprResult<evalexpr::Value> {
        // This match lists the custom functions available to laze evaluations.
        match identifier {
            "tr" => self.fn_tr(argument),
            "joinpath" => self.fn_joinpath(argument),
            "relroot" => self.fn_relroot(argument),
            _ => EvalexprResult::Err(evalexpr::EvalexprError::FunctionIdentifierNotFound(
                identifier.to_string(),
            )),
        }
    }

    fn are_builtin_functions_disabled(&self) -> bool {
        false
    }

    fn set_builtin_functions_disabled(&mut self, _disabled: bool) -> evalexpr::EvalexprResult<()> {
        EvalexprResult::Ok(())
    }
}

impl<'a, 'b: 'a> EvalContext<'a, 'b> {
    fn fn_relroot(&self, argument: &evalexpr::Value) -> Result<evalexpr::Value, EvalexprError> {
        use normalize_path::NormalizePath;
        let s = argument.as_string()?;
        let relroot = if let Some(relroot) = self.inner.get("relroot") {
            relroot
        } else {
            return EvalexprResult::Err(evalexpr::EvalexprError::VariableIdentifierNotFound(
                "relroot".into(),
            ));
        };
        let path = Utf8PathBuf::from(relroot).join(s);
        let path = path.as_std_path().normalize();
        EvalexprResult::Ok(evalexpr::Value::String(path.to_str().unwrap().into()))
    }

    fn fn_joinpath(&self, argument: &evalexpr::Value) -> Result<evalexpr::Value, EvalexprError> {
        let paths = argument.as_tuple()?;
        let mut result = Utf8PathBuf::new();
        for path in paths {
            result.push(path.as_string()?);
        }
        EvalexprResult::Ok(evalexpr::Value::String(result.into()))
    }

    fn fn_tr(&self, argument: &evalexpr::Value) -> Result<Value, EvalexprError> {
        // from Gemini Pro 3.1 (2026-03-13):
        fn tr_iterative(input: &str, from: &str, to: &str) -> String {
            input
                .chars()
                .map(|c| {
                    // Find the index of the current char in the 'from' set
                    if let Some(pos) = from.chars().position(|f| f == c) {
                        // If found, return the char at the same position in 'to'
                        // (Defaults to the original char if 'to' is shorter than 'from')
                        to.chars().nth(pos).unwrap_or(c)
                    } else {
                        // If not found, keep the original character
                        c
                    }
                })
                .collect()
        }

        let args = argument.as_tuple()?;
        if args.len() != 3 {
            return EvalexprResult::Err(evalexpr::EvalexprError::wrong_function_argument_amount(
                args.len(),
                3,
            ));
        }

        let input = args[0].to_string();
        let from = args[1].to_string();
        let to = args[2].to_string();

        if from.len() != to.len() {
            return EvalexprResult::Err(evalexpr::EvalexprError::CustomMessage(
                "from and to have different lengths".to_string(),
            ));
        }

        let result = tr_iterative(&input, &from, &to);

        EvalexprResult::Ok(Value::String(result))
    }
}

#[cfg(test)]
mod test {
    use crate::nested_env::{expand_eval, EnvMap, IfMissing};

    #[test]
    fn joinpath() {
        let vars = EnvMap::new();
        assert_eq!(
            expand_eval(r#"$(joinpath ("/foo", "bar"))"#, &vars, IfMissing::Error),
            Ok("/foo/bar".into())
        );
    }
}
