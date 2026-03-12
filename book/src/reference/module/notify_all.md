# notify_all

This field controls the content of the `${notify}` variable for this module.

Possible values: [`true`, `false`]. Defaults to `false`.

When `false`, `${notify}` contains only the names of this module's recursive
dependencies. When `true`, `${notify}` contains the names of ALL modules in
the build.

Module names in `${notify}` are converted to uppercase with `-`, `/`, `.`, and
`:` replaced by `_`.

The `${notify}` variable is typically used with
[`var_options`](../context/var_options.md) to generate preprocessor defines.

Example:

```yaml
builders:
  - name: my_builder
    var_options:
      notify:
        prefix: -DMODULE_

modules:
  - name: auto_init
    notify_all: true
    sources:
      - auto_init.c
```

With modules `auto_init` and `my_driver` in the build, `${notify}` for the
`auto_init` module would contain `MY_DRIVER AUTO_INIT`, and with the
`var_options` above, it flattens to `-DMODULE_MY_DRIVER -DMODULE_AUTO_INIT`.
