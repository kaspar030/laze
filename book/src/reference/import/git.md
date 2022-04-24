# `git`

A git import makes laze clone additional laze files from a git repository.

`git` needs a url and one of `commit`, `tag` or `branch`.

TODO: `tag` and `branch` are currently unimplmemented.

Example:

```yaml
imports:
 - git:
    url: https://example.com/foo
    commit: 890c1e8e02b0a872d73b1e49b3da45a5c8170016
    # or
    # tag: v1.2.3
    # or
    # branch: main
```
