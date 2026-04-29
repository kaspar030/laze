# Logging

Logging in Laze uses the common [log] facade with the associated macros.
Structured logging is not used.

The [env_logger] crate is used as frontend for the logger,
with a plain output configuration.

Log messages must be clear and plain language for the user (humans).
It must be clear for the user whether a messages is informational or an error.

## Log levels

The log level is set by the user of Laze via the command line or via
an environment variable (`LAZE_LOG_LEVEL`). Users can both silence logging or
enable all logging, with the default log level set to `Info`.

There are five log levels:

- Error
- Warn(ing)
- Info
- Debug
- Trace

`Error` has the highest priority (but lowest volume), `Trace` the lowest
priority (but highest volume).

Enabling a lower priority log level also implicitly enables all higher priority
log levels. E.g., setting the log level to `Trace` shows all log messages,
setting it to `Info` enables `Info`, `Warn` and `Error` log messages.
Setting the log level to `Error` only enables `Error` log messages.

When configured via the command line flags, the `--verbose` and `--quiet` flags
modify the output log level. These flags are treated as additive, the
difference between the number of `--verbose` and `--quiet` flags modifies the
default log level to more or less output.

### Error

Indicates a very serious error to the user. Fatal to the operation.
It must be clear for the user what went wrong.

Examples:

- Indications which input caused the issue
  (a laze file error or a command line argument error).
- Actions required to resolve the error.

## Warning

Indicates a *potential* issue that might lead to errors or unexpected behaviour.
Similar to the 'Error' level, it must be clear where the warning originates from
and what potential action can be taken to resolve the warning.

## Info

Information messages to the user indicate success on an operation.
These should be used sparingly as to not flood the user output.

Examples:

- Indications of overal success.
- Introduce long running tasks (should really use an activity indicator).

## Debug

Debug log level produces more information to give the user a detailed picture
of what laze is doing.

Allows the user to debug the `laze-project.yaml` file,
for example by showing the exact command for a task.

Examples:

- Outcomes of the parsed laze-project.yaml

## Trace

Trace provides even more verbose output. Usually only necessary for debugging
issues in laze files or the dependency resolution.

Examples:

- Detailed dependency resolution steps.

[log]: https://crates.io/crates/log
[env_logger]: https://crates.io/crates/env_logger
