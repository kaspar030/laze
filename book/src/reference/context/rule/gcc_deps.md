# gcc_deps

This field is used to enable Ninja's automatic header dependency tracking.

It takes a filename (usually containing `$out`) and will make Ninja read that
file as Makefile expecting to contain extra dependencies of the source file.

`laze` will use this to set `depfile = ...` combined with `deps = gcc` in the
generated Ninja build file.

See the [Ninja Manual](https://ninja-build.org/manual.html#ref_headers) for more
information.

Example:

```yaml
    rules:
      - name: CC
        description: CC ${out}
        in: "c"
        out: "o"
        gcc_deps: "$out.d"
        cmd: "${CC} -MD -MF $out.d ${CFLAGS} -c ${in} -o ${out}"
```
