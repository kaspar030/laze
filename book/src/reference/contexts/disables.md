### `disable`

List of modules that are disabled for builds in this context or any of
its children.

Example:

```yaml
context:
 - name: kids_birthday_party
   parent: birthday_party
   disables:
     - beer
```
