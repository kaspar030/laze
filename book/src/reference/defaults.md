# defaults

This field contains a _map_ that defines default properties for modules and/or
apps defined in the same file and its subdirectories.

The map keys are `"module"` and/or `"app"`. Each value is a module entry
whose fields are inherited by all modules (or apps) in the same file and any
files included via [`subdirs`](./subdirs.md). Defaults propagate down through
subdirectories and can be overridden at each level.

Any module field can be set in defaults, including
[`context`](./module/context.md), [`sources`](./module/sources.md),
[`env`](./module/env.md), [`depends`](./module/depends.md), and others.

Example:

```yaml
defaults:
  module:
    context: my_builder
    env:
      export:
        CFLAGS:
          - -Wall

apps:
  - name: my_app
    # inherits context: my_builder and CFLAGS from defaults
    sources:
      - main.c
```
