# name

Name of this module. Any UTF8 string is valid. This *will* be used as part of
file- and directory names, so better keep it simple.

Within each context, module names *must* be unique.

Each module *must* have a `name`. If the field is ommitted, [`${relpath}`](../variables.md#relpath)
is used.

Example:

```yaml
modules:
 - name: name_of_this_module.
   # ... possible other fields
 - name: name_of_other_module
   # ... possible other fields
```
