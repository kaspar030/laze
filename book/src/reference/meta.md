# meta

This field is ignored by laze and can contain arbitrary data.

It is available on the top level, on contexts, modules, rules, and tasks, and
can be used by external tools to attach metadata to laze build files.

Example:

```yaml
modules:
  - name: my_module
    meta:
      maintainer: alice
      license: MIT
```
