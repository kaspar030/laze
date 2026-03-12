# requires

This field contains an optional _list_ of module names that must be present in
the build for this module to be valid.

Unlike [`depends`](./depends.md), `requires` does not automatically select the
listed modules. The build fails if a required module is not selected by
something else.

Example:

```yaml
modules:
  - name: network_driver
    requires:
      - hardware_abstraction
```
