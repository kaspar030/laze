# provides

_List_ of features that this module _provides_.

If a module depends on or selects something _provided_ by another module, it
works like an alias.

A feature can be _provided_ by multiple modules. In that case, all providing
modules will be considered. Unless the dependency is optional, it fails if not
at least one module can be resolved. All modules that resolve will be used.

See also [`provides_unique`](./provides_unique.md)

Example:

```yaml
modules:
 - name: amazon_s3
   provides:
     - s3_api
   # ... possible other fields
 - name: backblaze_s3
   provides:
     - s3_api
   # ... possible other fields

 - name: s3_storage
   depends:
     # both "amazon_s3" and "backblaze_s3" will be added to dependencies
     - s3_api
```
