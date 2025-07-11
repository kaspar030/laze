use camino::Utf8PathBuf;
use evalexpr::{EvalexprError, EvalexprResult};

use super::EnvMap;

pub struct EvalContext<'a, 'b: 'a> {
    inner: &'a EnvMap<'b>,
}

impl<'a, 'b: 'a> EvalContext<'a, 'b> {
    pub fn new(env: &'a EnvMap<'b>) -> Self {
        Self { inner: env }
    }
}

impl evalexpr::Context for EvalContext<'_, '_> {
    fn get_value(&self, _identifier: &str) -> Option<&evalexpr::Value> {
        None
    }

    fn call_function(
        &self,
        identifier: &str,
        argument: &evalexpr::Value,
    ) -> evalexpr::EvalexprResult<evalexpr::Value> {
        // This match lists the custom functions available to laze evaluations.
        match identifier {
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
