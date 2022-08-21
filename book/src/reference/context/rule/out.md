# out

`out` is used to specify the extension of output files for this rule.

`laze` will use this to generate output file names.

Example:

```yaml
    rules:
      - name: CC
        description: CC ${out}
        in: "c"
        out: "o"
        # ... other fields ...
```
