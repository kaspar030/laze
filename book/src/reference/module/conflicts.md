# conflicts

_List_ of modules that this module _conflicts_.
Two conflicting modules cannot both be part of a build.

`A` conflicting `B` implies `B` conflicting `A`.

It is possible to _conflict_ a _provided_ feature to ensure a module is the
only selected module _providing_ a feature, but
[`provides_unique`](./provides_unique.md) *should* be used for this.

Example:

```yaml
modules:
 - name: gnu_libc
   conflicts:
     - musl_libc
   # ... possible other fields
```
