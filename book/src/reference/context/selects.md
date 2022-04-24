### `selects`

List of modules that are always selected for builds in this context or any of
its children.

Example:

```yaml
context:
 - name: birthday_party
   selects:
     - cake
     - music
```
