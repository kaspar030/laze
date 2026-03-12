# help

This field contains an optional _string_ with help text for the task.
It is used for CLI shell completion.

Example:

```yaml
    tasks:
      flash:
        help: "flash the compiled binary to the target device"
        cmd:
          - flash-tool ${out}
```
