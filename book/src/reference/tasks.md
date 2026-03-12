# tasks

This field contains a _map_ of named task entries.

Tasks are operations that are executed by laze (not ninja), after optionally
building the app binary. They can be used to execute a compiled binary, run
it through a debugger, flash an embedded target, and more.

Each map key is the task name, which is used to invoke the task from the command
line (e.g., `laze run`, `laze size`).

Tasks can be defined on contexts, builders, and modules. Tasks defined in parent
contexts are inherited by child contexts and builders. Module tasks can override
context tasks of the same name. If two modules define a task with the same name,
they are treated as conflicting and cannot both be selected in the same build.

Example:

```yaml
contexts:
  - name: default
    tasks:
      info:
        cmd:
          - "echo binary: ${out}"
      size:
        cmd:
          - "${SIZE} ${out}"

builders:
  - name: host
    tasks:
      run:
        cmd:
          - ${out}
```

Tasks on modules:

```yaml
modules:
  - name: my_module
    tasks:
      test:
        cmd:
          - ${out} --run-tests
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
- [`meta`](./meta.md)
