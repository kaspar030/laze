# provides_unique

_List_ of features that this module provides, but needs to be the only provider.

`provides_unique` can be used for this.

Adding to `provides_unique` is equivalent to adding to both `provides` and
`conflicts`.

Example:

```yaml
modules:
 - name: gnu_libc
   # ensure only one "libc" will be chosen
   provides_unique:
     - libc
 - name: musl_libc
   provides_unique:
     - libc
   # ... possible other fields
```
