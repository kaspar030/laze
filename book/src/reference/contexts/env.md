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
