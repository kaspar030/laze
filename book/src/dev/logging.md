# Logging

Logging in Laze uses the common [log] facade with the associated macros.
Structured logging is not used.

The [env_logger] crate is used as frontend for the logger,
with a plain output configuration.

Log messages must be clear and plain language for the user (humans).
It must be clear for the user wether a messages is informational or an error.

## Log levels

- Error
- Warn(ing)
- Info
- Debug
- Trace

### Error

Indicates a very serious error to the user. Fatal to the operation.
It must be clear for the user what went wrong.

- preferably indicating which input caused the issue
  (a laze file error or a command line argument error).
- Actions required to resolve the error.

## Warning

Indicates a *potential* issue that might lead to errors or unexpected behaviour.
Similar to the 'Error' level, it must be clear where the warning originates from
and what potential action can be taken to resolve the warning.

## Info

Information messages to the user indicate success on an operation.
These should be used sparingly as to not flood the user output.

- Indications of overal success.
- introduce long running tasks (should really use an activity indicator).

## Debug

Debug log level produces more information for the user
when they have to figure out issues in their laze file.
Allows the user to debug the `laze-project.yaml` file,
for example by showing the exact command for a task.

- Outcomes of the parsed laze-project.yaml

## Trace

Trace provides detailed operation of the internals of Laze.
Trace is the lowest level and produces the most output.
Users should not need this level at all
and it is only useful to developers of laze.

- Internals of laze
- Statistics

[log]: https://crates.io/crates/log
[env_logger]: https://crates.io/crates/env_logger
