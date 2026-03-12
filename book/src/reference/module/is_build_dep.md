# is_build_dep

This field controls whether modules that depend on or use this module wait for
its build outputs before compiling.

Possible values: [`true`, `false`]. Defaults to `false`.

This is useful for modules with a custom [`build`](./build.md) step that
generates headers or source files needed by other modules.

Modules with [`download`](./download.md) automatically have `is_build_dep` set
to `true`.

Example:

```yaml
modules:
  - name: code_generator
    is_build_dep: true
    build:
      cmd:
        - generate-headers --out build/${builder}/generated.h
      out:
        - build/${builder}/generated.h
```
