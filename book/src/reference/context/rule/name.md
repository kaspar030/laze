# name

This field contains the name for this rule.

It will be used as rule name (or rule name prefix) in the generated Ninja build
file, so use short, capital names/numbers/underscores like "CC", "CXX", ...

Some names have special meaning in `laze`:

- `LINK` is the rule used to combine compiled source files for a given application.
- `GIT_DOWNLOAD` will be used for downloading source files
- `GIT_PATCH` will be used for applying patches on git repositories.

Example:

```yaml
context:
 - # ...
   rules:
   - name: CC
   # ... possible other fields
```
