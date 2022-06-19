# name

Name of this module. Any UTF8 string is valid. This *will* be used as part of
file- and foldernames, so better keep it simple.

Within each context, module names *must* be unique.

Each module *must* have a `name` field, all other fields are optional.

Example:

```yaml
modules:
 - name: name_of_this_module.
   # ... possible other fields
 - name: name_of_other_module
   # ... possible other fields
```
