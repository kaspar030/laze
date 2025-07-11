# cmd

This field contains a _list_ of shell commands to execute sequentially.
Each entry is run as a shell command (`sh -c` on Unix, `cmd /C` on Windows).

Commands prefixed with `:` are treated as calls to other tasks. For example,
`:echo hello` calls the `echo` task with `hello` as argument.

Task commands support argument substitution: `$1`..`$N` for positional
arguments, `$*` for all arguments as a single string, and `$@` for all
arguments as individual arguments.

Laze variables (e.g., `${out}`, `${builder}`) are expanded in commands.

Example:

```yaml
    tasks:
      info:
        cmd:
          - "echo binary: ${out}"
          - "echo builder: ${builder}"
      echo-all:
        cmd:
          - :echo first=$1
          - :echo all=$*
```
