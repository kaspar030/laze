# var_options

This field contains a _map_ that controls how variables are flattened into
strings during the build process.

Each map key is a variable name. The value is a set of formatting options
that are applied when the variable's list of values is joined into a single
string.

## options

| Option   | Type     | Default | Description                                            |
|----------|----------|---------|--------------------------------------------------------|
| `from`   | _string_ | -      | Read values from another variable instead of this one  |
| `joiner` | _string_ | `" "`  | String used to join list elements                      |
| `prefix` | _string_ | -      | String prepended before each element                   |
| `suffix` | _string_ | -      | String appended after each element                     |
| `start`  | _string_ | -      | String prepended once before the entire result         |
| `end`    | _string_ | -      | String appended once after the entire result           |

Using `from` creates a new variable from another variable's values. A variable
cannot have both its own values and a `from` option.

## Examples

Adding a prefix to each element:

```yaml
builders:
  - name: my_builder
    var_options:
      notify:
        prefix: -DMODULE_
    env:
      notify:
        - apple
        - banana
```

This flattens `notify` to `-DMODULE_apple -DMODULE_banana`.

Using a custom joiner and wrapping the result:

```yaml
contexts:
  - name: default
    var_options:
      includes:
        joiner: ","
        prefix: "-I"
        start: "("
        end: ")"
    env:
      includes:
        - src
        - lib
```

This flattens `includes` to `(-Isrc,-Ilib)`.

Creating a new variable from an existing one:

```yaml
contexts:
  - name: default
    var_options:
      module_defines:
        from: modules_used
        prefix: -DMODULE_
```

This creates `module_defines` by reading the values of `modules_used` and
prefixing each element with `-DMODULE_`.
