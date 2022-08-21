# in

`in` is used to specify the extension of input files for this rule.

`laze` will look up a rule for each source file depending on this field.

Example:

```yaml
    rules:
      - name: CC
        description: CC ${out}
        in: "c"
        # ... other fields ...
```
