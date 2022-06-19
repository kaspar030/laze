# depends

_List_ of modules this module depends on.

If a module depends on another module, it will pull that module into the build,
and import that module's exported _environment_.

Note: _depending_ a module is equivalent to both _selecting_ and _using_ it.

If a dependency name is prefixed with "?", the dependency turns into an optional
dependency. That means, if the dependency is available, it will be depended on,
otherwise it will be ignored.

Example:

```yaml
modules:
 - name: datetime
   depends:
     - date
     - time
   # ... possible other fields
 - name: party
   depends:
     - people
     - ?music
     - ?alcohol
   # ... possible other fields
```
