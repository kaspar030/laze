# srcdir

This field contains an optional _string_ that overrides the base path for the
module's source files.

By default, `srcdir` is the module's [`relpath`](../variables.md#relpath) (the
directory containing the laze YAML file), or the download directory for modules
with [`download`](./download.md).

The value is also available as the `${srcdir}` variable in the module's
environment.

Example:

```yaml
modules:
  - name: vendor_lib
    srcdir: vendor/lib/src
    sources:
      - lib.c
      - util.c
```
