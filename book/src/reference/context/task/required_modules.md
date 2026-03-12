# required_modules

This field contains an optional _list_ of module names that must be selected
for the task to be available in a build configuration.

Example:

```yaml
    tasks:
      flash:
        required_modules:
          - flasher_support
        cmd:
          - flash-tool ${out}
```
