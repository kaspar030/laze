# builders

`builders` contains a _list_ of builder entries.

Example:

```yaml
builders:
 - name: name_of_this_builder.
   # ... possible other fields
 - name: name_of_other_builder
   # ... possible other fields
```

## builder fields

As a builder is just a special kind of context, they share all fields.

- [`name`](./context/name.md)
- [`parent`](./context/parent.md)
- [`env`](./context/env.md)
- [`selects`](./context/selects.md)
- [`disables`](./context/disables.md)
- [`rules`](./context/rules.md)
- [`var_options`](./context/var_options.md)
- [`tasks`](./context/tasks.md)
