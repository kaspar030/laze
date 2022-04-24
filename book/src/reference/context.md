# context

`context` contains a _list_ of context entries.
Each context *must* have a `name` field, all other fields are optional.

Example:

```yaml
context:
 - name: name_of_this_context.
   # ... possible other fields
 - name: name_of_other_context
   # ... possible other fields
```

## context fields

### `parent`

The parent of this context. If unset, defaults to `default`.

Example:

```yaml
context:
 - name: some_name
   parent: other_context_name
```

### `env`

A _map_ of variables.
Each map key correspondents to the variable name.
Each value must be either a single value or a list of values.

Example:

```yaml
context:
 - name: some_name
   env:
     CFLAGS:
       - -D_this_is_a_list
       - -D_of_multiple_values
     LINKFLAGS: -Ithis_is_a_single_value
```

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

### `rule`

TODO

### `var_options`

TODO

### `tasks`

TODO
