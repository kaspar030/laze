# provides_unique

This field contains an optional _list_ of virtual module names that this
context uniquely provides.

Like [`provides`](./provides.md), but also adds the listed names to
`conflicts`. This means only one context (or module) that provides the same
name via `provides_unique` can be active in a build.

Example:

```yaml
contexts:
  - name: context_uart
    parent: default
    provides_unique:
      - serial_backend

  - name: context_usb
    parent: default
    provides_unique:
      - serial_backend
```

In this example, a build cannot use both `context_uart` and `context_usb`
since they both uniquely provide `serial_backend`.
