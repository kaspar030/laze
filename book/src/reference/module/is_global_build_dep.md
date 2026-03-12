# is_global_build_dep

This field controls whether ALL modules in the build wait for this module's
build outputs before compiling.

Possible values: [`true`, `false`]. Defaults to `false`.

This is a stronger variant of [`is_build_dep`](./is_build_dep.md). While
`is_build_dep` only affects direct dependees, `is_global_build_dep` affects
every module in the entire build.

Example:

```yaml
modules:
  - name: global_config_generator
    is_global_build_dep: true
    build:
      cmd:
        - generate-config > build/${builder}/config.h
      out:
        - build/${builder}/config.h
```
