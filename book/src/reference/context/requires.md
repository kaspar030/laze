# requires

This field contains an optional _list_ of module names that must be present
when this context is used.

Unlike [`selects`](./selects.md), `requires` does not automatically select the
listed modules. The build fails if a required module is not selected by
something else.

Example:

```yaml
contexts:
  - name: secure_context
    parent: default
    requires:
      - crypto_support
```
