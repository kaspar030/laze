# provides

_List_ of modules that this module _conflicts_.
Two conflicting modules cannot both be part of a build.

`A` conflicting `B` implies `B` conflicting `A`.

It is possible to _conflict_ a _provided_ feature to ensure a module is the only
selected module _providing_ a feature.

Example:

```yaml
modules:
 - name: gnu_libc
   # only one libc can be linked in, so `provide` _and_ `conflict` `libc`.
   provides:
     - libc
   conflicts:
     - libc
   # ... possible other fields
```
