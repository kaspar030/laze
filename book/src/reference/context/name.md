### `name`

Name of this context. Any UTF8 string is valid. This *will* be used as part of
file- and directory names, so better keep it simple.

Context names *must* be unique.

Each context *must* have a `name` field, all other fields are optional.

Example:

```yaml
contexts:
 - name: name_of_this_context.
   # ... possible other fields
 - name: name_of_other_context
   # ... possible other fields
```
