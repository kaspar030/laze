# pool

This allows limiting execution of this rule's to named concurrency pools.
See the [Ninja Manual](https://ninja-build.org/manual.html#ref_pool) for more
information.

Currently, the only supported pool is Ninja's predefined `console` pool.

From the Ninja manual:

```
It has the special property that any task in the pool has direct access to the
standard input, output and error streams provided to Ninja, which are normally
connected to the user’s console (hence the name) but could be redirected. This
can be useful for interactive tasks or long-running tasks which produce status
updates on the console (such as test suites).

While a task in the console pool is running, Ninja’s regular output (such as
progress status and output from concurrent tasks) is buffered until it
completes.
```

Example:

```yaml
    rules:
      - name: FOO
        pool: console
        cmd: some_command_that_needs_console_input
```
