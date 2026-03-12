# download

This field specifies a git repository to download the module's source files
from.

Modules with `download` automatically have [`is_build_dep`](./is_build_dep.md)
set to `true`, and their [`srcdir`](./srcdir.md) points to the download
directory.

The context must define `GIT_DOWNLOAD` and (if using patches) `GIT_PATCH` rules.

## fields

- `git`: Required. A _map_ with `url` and one of `commit`, `branch`, or `tag`.
- `patches`: Optional. A _list_ of patch files to apply after download.
- `dldir`: Optional. A _string_ overriding the download directory. Defaults to
  `build/dl/<relpath>/<module_name>`.

Example:

```yaml
modules:
  - name: external_lib
    download:
      git:
        url: https://github.com/example/lib.git
        commit: abc123
    sources:
      - lib.c

  - name: patched_lib
    download:
      git:
        url: https://github.com/example/lib.git
        commit: main
      patches:
        - 0001-fix-build.patch
    sources:
      - lib.c
```
