# required_vars

This field contains an optional _list_ of variable names that must be set
for the task to be available in a build configuration.

Example:

```yaml
    tasks:
      flash:
        required_vars:
          - FLASH_TOOL
          - TARGET_PORT
        cmd:
          - ${FLASH_TOOL} -p ${TARGET_PORT} ${out}
```
