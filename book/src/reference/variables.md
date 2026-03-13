# Variables defined by laze

## appdir

This variable contains the path in which the app of a build was defined.

Useful mostly for tasks, as `${relpath}` would evaluate to the directory in which
the task is defined.

Examples:

```yaml
# in apps/foo/laze.yml
apps:
  - name: foo_app


# in modules/foo/laze.yml
modules:
  - name: foo
    env:
      export:
        CFLAGS:
          - -DAPPDIR=\"${appdir}\"
```

> [!WARNING]
> Variables without braces are not evaluated by Laze,
> and are retained as is in the command.

## builder

This variable contains the name of the builder.

Examples:

```yaml
modules:
  - name: foo
    env:
      export:
        CFLAGS:
          - -DBUILDER=\"${builder}\"
```

## modules

This variable evaluates to a list of all modules used in a build.

## outfile

This variable contains the full path to the linked output binary of a build.
Defaults to `${bindir}/${app}.elf`.

Can be overridden in `env:` to change the output filename or extension.
The final resolved value becomes available as `${out}` in tasks.

If a `POST_LINK` rule is defined, its `out` extension replaces the original
extension (e.g., `.elf` becomes `.bin`).

Example:

```yaml
builders:
  - name: my_builder
    env:
      outfile: "${bindir}/${app}.bin"
```

## relpath

This variable is evaluated early and will be replaced with the relative (to the
project root) path of the laze yaml file.

Example:

```yaml
modules:
  - name: foo
    env:
      export:
        CFLAGS:
          - -I${relpath}/include
```

## root

This variable will be replaced with the relative path to the root of the main
project. This can be used for specifying root-relative path names.
Usually (for laze projects that were not imported), this contains `.`
If a laze file is part of an import of another laze project, `${root}` contains
the relative path to the location where the import has been downloaded.

Example:

```yaml
# in some project:
imports:
  - git:
     url: .../foo.git
     commit: ...


# in .../foo.git/some/subdir/laze.yml:

modules:
  - name: foo
    env:
      export:
        CFLAGS:
          - -I${root}/include
          # this will evaluate to `-Ibuild/imports/foo-<hash>/include`
```

## srcdir

Contains the relative (to project root) base path of a module's source files.
If a module has not been downloaded, this is usually identical to `${relpath}`.
For downloaded modules, it points to the module's download directory.

## app

This variable contains the name of the app being built.

Example:

```yaml
modules:
  - name: foo
    env:
      export:
        CFLAGS:
          - -DAPP_NAME=\"${app}\"
```

## build-dir

This variable contains the top-level build output directory. Defaults to
`build`, and can be changed with the `--build-dir` / `-B` CLI flag.

## contexts

This variable evaluates to a list of all context names in the current builder's
context chain, from root to builder.

## LAZE_BIN

This variable contains the absolute path to the currently running laze binary.
Used internally by download rules.

## notify

This variable contains a list of module names relevant to the current module.
By default, it contains only the names of recursive dependencies. If
[`notify_all`](./module/notify_all.md) is set to `true`, it contains all
modules in the build.

Module names are converted to uppercase with `-`, `/`, `.`, and `:` replaced
by `_`.

Typically used with [`var_options`](./context/var_options.md) to generate
preprocessor defines.

## out

In build rules, this variable passes through to ninja's `$out` variable.

In tasks, this variable contains the final output binary path (the resolved
value of [`outfile`](#outfile), possibly with a changed extension from a
`POST_LINK` rule).

## project-root

This variable contains the path to the project root directory.

## relroot

This variable contains the relative path from the current module's location
back to the project root. For example, a module at `src/drivers/` would have
`relroot` set to `../..`.
