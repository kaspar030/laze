# build

This field controls whether the app binary is built (via ninja) before
executing the task.

Possible values: [`true`, `false`]. Defaults to `true`.

Set to `false` for tasks that don't need a compiled binary, such as
cleaning up or printing configuration information.

Example:

```yaml
    tasks:
      clean:
        build: false
        cmd:
          - rm -rf ${bindir}
```
