# rules

This fiels contains a _list_ of laze build rules.

A laze build rule mostly correspondents to Ninja rules.

Example:

```yaml
contexts:
  - name: default
    rules:
      - name: CC
        in: c
        out: o
        cmd: ${CC} ${in} -o ${out}
```

## rule fields

- [`name`](./rule/name.md)
- [`description`](./rule/description.md)
- [`cmd`](./rule/name.md)
- [`in`](./rule/in.md)
- [`out`](./rule/out.md)
- [`options`](./rule/option.md)
- [`gcc_deps`](./rule/gcc_deps.md)
- [`rspfile`](./rule/rspfile.md)
- [`rspfile_content`](./rule/rspfile_content.md)
- [`pool`](./rule/pool.md)
- [`always`](./rule/always.md)
