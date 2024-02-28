# builders

`builders` contains a _list_ of builder entries.

A `builder` represents a configuration and set of available modules that an
[`app`](./apps.md) can be built for.

Example:

```yaml
builders:
 - name: name_of_this_builder.
   # ... possible other fields
 - name: name_of_other_builder
   # ... possible other fields
contexts:
 - name: name_of_another_builder
   buildable: true
   # ... possible other fields
```

## builder fields

As a builder is just a context that has `buildable: true` set, they share all fields.

- [`name`](./context/name.md)
- [`parent`](./context/parent.md)
- [`env`](./context/env.md)
- [`selects`](./context/selects.md)
- [`disables`](./context/disables.md)
- [`rules`](./context/rules.md)
- [`var_options`](./context/var_options.md)
- [`tasks`](./context/tasks.md)
