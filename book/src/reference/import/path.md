# `path`

A `path` import allows using a local directory as import source.

If `path` is not absolute, it is considered relative to the project root.

Optionally, `path` can be symlinked into the `imports` folder inside the laze
build directory by setting `symlink: true` in the import.
The symlink name within `$build_dir/imports` defaults to the last path component
of `path`. This can be changed by setting `name`.
Using a symlink helps turning absolute pathnames into relative ones. This might
be desirable for privacy reasons, or help with reproducible builds.

Example:

```yaml
imports:
  - path: /path/to/local/folder
  - path: /path/to/another/local/folder
    symlink: true
  - path: /path/to/a/third/local/folder
    symlink: true
    name: folder3
```