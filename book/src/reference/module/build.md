# build

This field defines a custom build step for the module.

It is used for code generation or pre-processing steps where a module needs to
produce source files or other artifacts before normal compilation. The build
commands have access to the module's environment variables.

## fields

- `cmd`: Required. A _list_ of shell commands. They are joined with ` && ` to
  form a single ninja build command.
- `out`: Optional. A _list_ of output files produced by the custom build.
- `gcc_deps`: Optional. A _string_ specifying a file for gcc-style dependency
  tracking.

Example:

```yaml
modules:
  - name: generated_code
    is_build_dep: true
    depends:
      - config_module
    build:
      cmd:
        - echo ${VARIABLE} > build/${builder}/generated.c
      out:
        - build/${builder}/generated.c
    sources:
      - build/${builder}/generated.c
```
