# ignore_ctrl_c

This field controls whether Ctrl+C (SIGINT) signals are ignored during
task execution.

Possible values: [`true`, `false`]. Defaults to `false`.

This is useful for tasks that run interactive programs which handle
Ctrl+C themselves, such as debuggers or serial monitors.

Example:

```yaml
    tasks:
      debug:
        ignore_ctrl_c: true
        cmd:
          - gdb ${out}
```
