# cmd

This field contains the command to be run when the output needs to be rebuilt.
Will end up in Ninja's "command" field for this rule.

Example:

```yaml
    rules:
      - name: CC
        cmd: "${CC} -c ${in} -o ${out}""
        # ... other fields ...
```
