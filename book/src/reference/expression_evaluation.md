# Expression evaluation

Laze supports inline expression evaluation using the `$(...)` syntax.

Expressions are evaluated after variable expansion (`${...}`), so variables are
resolved first, then expressions are evaluated.

Expression evaluation is powered by the
[evalexpr](https://crates.io/crates/evalexpr) crate.

## Syntax

- `$(expression)` evaluates the expression and substitutes the result.
- Nesting is supported: inner expressions are evaluated first.
- `$$(...)` escapes the dollar sign and is not evaluated.

## Examples

Arithmetic:

```yaml
env:
  RESULT: "$(1 + 1)"
  # evaluates to "2"
```

Nested expressions:

```yaml
env:
  RESULT: "$(1 + $(1 + 1))"
  # evaluates to "3"
```

Functions:

```yaml
env:
  MAX_VAL: "$(max(1, 2, 3, 4))"
  # evaluates to "4"
```

String functions:

```yaml
env:
  UPPER: '$(str::to_uppercase "foobar")'
  # evaluates to "FOOBAR"
```

Combined with variable expansion:

```yaml
env:
  BASE: "10"
  OFFSET: "$(${BASE} + 5)"
  # evaluates to "15"
```

## Supported operations

- Arithmetic: `+`, `-`, `*`, `/`, `%`, `^`
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Boolean: `&&`, `||`, `!`
- String concatenation: `+` (on strings)
- Conditional: `if(condition, then, else)`

## Supported functions

Math: `min`, `max`, `floor`, `ceil`, `round`, `abs`, `sqrt`, `exp`, `ln`, `log`

String: `str::to_uppercase`, `str::to_lowercase`, `str::trim`,
`str::contains`, `str::starts_with`, `str::ends_with`, `str::len`,
`str::from`, `str::substring`, `str::regex_matches`, `str::regex_replace`
