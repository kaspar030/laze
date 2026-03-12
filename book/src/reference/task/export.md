# export

This field contains an optional _list_ of variables to export to the task's
shell environment.

Each entry can be either a bare variable name or a key-value _map_.
A bare variable name exports the corresponding laze variable's value.
A key-value map exports the given value under the given name.

Exported variables are also available to subtasks called via the `:` prefix.

Example:

```yaml
    tasks:
      deploy:
        export:
          - CFLAGS
          - TARGET_ADDR: 192.168.1.42
        cmd:
          - deploy-tool --addr $TARGET_ADDR ${out}
```
