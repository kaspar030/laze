### `buildable`

Set if this `context` if `apps` can be built for this context.
Possible values: [`true`, `false`]. Defaults to `false`.

Example:

```yaml
context:
 - name: kids_birthday_party
   parent: birthday_party
   buildable: true
```
