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

- [`name`](./context/name.md)
- [`parent`](./context/parent.md)
- [`env`](./context/env.md)
- [`selects`](./context/selects.md)
- [`disables`](./context/disables.md)
- [`rules`](./context/rules.md)
- [`var_options`](./context/var_options.md)
- [`tasks`](./context/tasks.md)
