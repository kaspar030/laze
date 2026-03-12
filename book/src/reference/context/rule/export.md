# export

This field contains an optional _list_ of variables to export to the rule's
build environment.

Each entry can be either a bare variable name or a key-value _map_.
A bare variable name exports the corresponding laze variable's value.
A key-value map exports the given value under the given name.

Example:

```yaml
    rules:
      - name: CC
        cmd: "${CC} -c ${in} -o ${out}"
        export:
          - CFLAGS
          - CC_VERSION: 12
        # ... other fields ...
```
