# includes

This field contains a _list_ of YAML file paths to include.

Paths are relative to the directory of the current file. Each included file is
loaded and processed as if its content were part of the including file. Included
files share the same `relpath` context as the including file.

Unlike [`subdirs`](./subdirs.md), which looks for `laze.yml` in subdirectories
and adjusts `relpath`, `includes` loads files directly without changing the
path context.

Example:

```yaml
# in laze.yml
includes:
  - extra-modules.yml
  - generated.yml

apps:
  - name: my_app
```
