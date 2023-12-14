# laze_required_version

Expects a semver version string (`a.b.c`). Laze will refuse to read the file if
its own version is smaller.

Example:

```yaml
laze_required_version: 1.0.0
```
