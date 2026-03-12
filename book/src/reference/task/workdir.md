# workdir

This field contains an optional _string_ specifying the working directory for
the task's commands. If not set, commands run in the application's build
directory.

A relative path is resolved relative to the application's build directory.
An absolute path is used as-is.

Example:

```yaml
    tasks:
      test:
        workdir: ../test-fixtures
        cmd:
          - ./run-tests.sh ${out}
```
