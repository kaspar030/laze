# rspfile

Use this to specify a file containing additional command line arguments.
Supposed to be used in combination with [`rspfile_content`](./rspfile_content.md).

This can be used if resulting command lines get too long, and the used tool
supports reading arguments from file.

Example:

```yaml
    rules:
      - name: LINK
        in: 'o'
        rspfile: $out.rsp
        rspfile_content: $in
        cmd: ${LINK} @${out}.rsp -o ${out}
```
