# always

This boolean flag makes the rule always run. It basically makes the resulting
ninja build entry "phony".

Currently, this only has any effect for the special `LINK` rule.
