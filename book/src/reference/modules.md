# modules

'modules' contains a _list_ of module entries.

Example:

```yaml
modules:
 - name: name_of_this_module.
   # ... possible other fields
 - name: name_of_other_context
   # ... possible other fields
```

## module fields

- [`name`](./module/name.md)
- [`context`](./module/context.md)
- [`help`](./module/help.md)
- [`sources`](./module/sources.md)
- [`env`](./module/env.md)
- [`depends`](./module/depends.md)
- [`selects`](./module/selects.md)
- [`uses`](./module/uses.md)
- [`conflicts`](./module/conflicts.md)
- [`provides`](./module/provides.md)
- [`provides_unique`](./module/provides_unique.md)
- [`requires`](./module/requires.md)
- [`build`](./module/build.md)
- [`download`](./module/download.md)
- [`tasks`](./tasks.md)
- [`notify_all`](./module/notify_all.md)
- [`srcdir`](./module/srcdir.md)
- [`is_build_dep`](./module/is_build_dep.md)
- [`is_global_build_dep`](./module/is_global_build_dep.md)
- [`meta`](./meta.md)
