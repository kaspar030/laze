use std::borrow::Cow;

use evalexpr::EvalexprError;

pub fn eval<'a>(input: &'a str) -> Result<Cow<'a, str>, EvalexprError> {
    if input.contains("$(") {
        eval_recursive(input, false)
    } else {
        Ok(input.into())
    }
}

fn eval_recursive<'a>(input: &'a str, is_eval: bool) -> Result<Cow<'a, str>, EvalexprError> {
    let mut result = String::new();
    let mut start = 0;
    let mut level = 0;
    let mut input_changed = false;

    for (i, character) in input.char_indices() {
        if character == '$'
            && i + 1 < input.len()
            && input[i + 1..i + 2] == *"("
            && (i == 0 || (input[i - 1..i] != *"$"))
        {
            if level == 0 {
                start = i + 1;
            }
        } else if character == '(' && start > 0 {
            level += 1;
        } else if character == ')' && level > 0 && start > 0 {
            level -= 1;
            if level == 0 {
                input_changed = true;
                result.push_str(&eval_recursive(&input[start + 1..i], true)?);
            }
        } else if level == 0 {
            result.push(character);
        }
    }

    if is_eval {
        let expr = evalexpr::eval(&result)?.to_string();
        input_changed = true;
        result = expr;
    }

    if input_changed {
        Ok(result.into())
    } else {
        Ok(input.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let result = eval("foo $(1+$(1+1)) after_foo");
        assert_eq!(result.unwrap(), "foo 3 after_foo");
    }
    #[test]
    fn nested_braces() {
        let result = eval("$((0))");
        assert_eq!(result.unwrap(), "0");
    }
    #[test]
    fn basic_eval_max() {
        let result = eval("$(max(1,2,3,4))");
        assert_eq!(result.unwrap(), "4");
    }
    #[test]
    fn basic_eval_add() {
        let result = eval("$(str::to_uppercase \"foobar\")");
        assert_eq!(result.unwrap(), "\"FOOBAR\"");
    }
    #[test]
    fn unchanged() {
        let result = eval("just some text");
        assert_eq!(result.unwrap(), Cow::Borrowed("just some text"));
    }
    #[test]
    fn escaped_dollar() {
        let literal = "just some $$(foo) text";
        let result = eval(literal);
        assert_eq!(result.unwrap(), Cow::Borrowed(literal));
    }
}
