# env

This field contains a _map_ of environment variable scopes for the module.

Three scopes are available:

- `local`: Variables only visible to this module's own build commands. Applied
  after exported variables from dependencies, so it can override imported values.
- `export`: Variables visible to this module and transitively to all modules
  that depend on or use this module.
- `global`: Variables merged into the build's global environment, visible to
  all modules in the build.

Each scope is a _map_ where keys are variable names and values are either a
single value or a list of values.

Example:

```yaml
modules:
  - name: my_module
    env:
      local:
        CFLAGS:
          - -DLOCAL_ONLY
      export:
        CFLAGS:
          - -I${relpath}/include
      global:
        GLOBAL_VAR: my_module_was_here
```
