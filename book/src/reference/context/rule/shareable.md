# shareable

This boolean flag controls output sharing. It defaults to `true`.
See [object sharing][object-sharing] for details.

When enabled for a rule, this rule's output files will end up in a location
that is context-independent:

    ${build-dir}/objects/<filestem>.<hash>.<ext>

E.g., a source file `some-module/foo.c` would end up in `${build-dir}/objects/some-module/foo.<hash>.o`.

When disabled, the output will end up in a builder and app specific location:

    ${bindir}/<file-path>/foo.o

The variable `${bindir}` defaults to `${build-dir}/out/${builder}/${app}`, so
e.g., a source file `some-module/foo.c` built for builder `host` and app `bar`
would end up in `${bindir}/out/host/bar/some-module/foo.o`.

Example:

```yaml
context:
 - # ...
   rules:
   - name: MYRULE
     shareable: false # disable object sharing for this rule
   # ... possible other fields
```

[object-sharing]: ../../../concepts/object_sharing.md
