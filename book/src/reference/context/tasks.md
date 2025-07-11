# tasks

This field contains a _map_ of named task entries.

Tasks are operations that are executed by laze (not ninja), after optionally
building the app binary. They can be used to execute a compiled binary, run
it through a debugger, flash an embedded target, and more.

Each map key is the task name, which is used to invoke the task from the command
line (e.g., `laze run`, `laze size`).

Example:

```yaml
contexts:
  - name: default
    tasks:
      info:
        cmd:
          - "echo binary: ${out}"
      run:
        cmd:
          - ${out}
```

## task fields

- [`cmd`](./task/cmd.md)
- [`build`](./task/build.md)
- [`help`](./task/help.md)
- [`export`](./task/export.md)
- [`ignore_ctrl_c`](./task/ignore_ctrl_c.md)
- [`required_vars`](./task/required_vars.md)
- [`required_modules`](./task/required_modules.md)
- [`workdir`](./task/workdir.md)
