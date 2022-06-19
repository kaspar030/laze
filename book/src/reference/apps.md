# app

`apps` contains a _list_ of app entries.

Example:

```yaml
apps:
 - name: hello-world
   # ... possible other fields
 - name: foobar
   # ... possible other fields
```

## app fields

As an app is just a special kind of module, they share all fields.

- [`name`](./module/name.md)
- [`sources`](./module/sources.md)
- [`env`](./module/env.md)
- [`depends`](./module/depends.md)
- [`selects`](./module/selects.md)
- [`uses`](./module/uses.md)
- [`conflicts`](./module/conflicts.md)
- [`provides`](./module/provides.md)
- [`build`](./module/build.md)
