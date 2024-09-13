# `imports`

`imports` contains a _list_ of import entries, each representing a local or
remote source for additional laze projects.

Example:

```yaml
imports:
 - git:
    - url: https://github.com/kaspar030/laze
      tag: 0.1.17
```

## `import` types

- [`git`](./import/git.md)
- [`laze`](./import/laze.md)
- [`path`](./import/path.md)
