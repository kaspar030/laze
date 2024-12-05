# Variables defined by laze

## appdir

This variable contains the path in which the app of a build was defined.

Useful mostly for tasks, as `${relpath}` would evaluate to the folder in which
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
For downloaded modules, it points to the module's download folder.
