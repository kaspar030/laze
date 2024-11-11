# `git`

A git import makes laze clone additional laze files from a git repository.

`git` requires a url. Additionally, `commit`, `tag` or `branch` can be set. If
neither is specified, the default branch will be checked out.

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
