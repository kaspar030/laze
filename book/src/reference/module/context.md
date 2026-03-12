# context

This field specifies which context(s) a module belongs to.

Can be a single _string_ or a _list_ of strings. If omitted, the module belongs
to the `"default"` context.

When a list is given, the module is available in each of the specified contexts
(and their children).

Example:

```yaml
modules:
  - name: platform_support
    context: linux

  - name: shared_module
    context:
      - builder_arm
      - builder_riscv
```
