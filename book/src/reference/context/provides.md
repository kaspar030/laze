# provides

This field contains an optional _list_ of virtual module names that this
context provides.

This works the same as [`provides`](../module/provides.md) on modules. The
listed names are set on the context's associated module, making the context
satisfy dependencies on those names.

Example:

```yaml
contexts:
  - name: context_with_hw_support
    parent: default
    provides:
      - hardware_abstraction
```
