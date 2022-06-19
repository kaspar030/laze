# uses

_List_ of modules this module _uses_.

If a module _uses_ another module, it will import that module's exported
environment, if that module is part of the build.

Example:

```yaml
modules:
 - name: datetime
   uses:
     - timezone_support
   # ... possible other fields
```
