# rspfile_content

Use this to specify the contents of an [`rspfile`](./rspfile.md).

Example:

```yaml
    rules:
      - name: LINK
        in: 'o'
        rspfile: $out.rsp
        rspfile_content: $in
        cmd: ${LINK} @${out}.rsp -o ${out}
```
