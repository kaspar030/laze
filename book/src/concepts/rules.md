# Rules

laze needs to know how to compile sources, and how to link compiled sources.
Eventually, it needs to create a Ninja build file, and Ninja needs [Build Rules][NinjaRules]
to know how to build.

Currently, laze is bit focused on building C projects (with some C++ support),
so it expects rules that turn sources to object files that then can be linked
to form the final binary.

In laze, *rules* are part of *contexts*. When configuring a build, laze builds
a set of all rules of a builder and its ancestors. If names clash, children's
rules have precedence over parents.

laze iterates the sources of each module, and then looks up a rule *based on
the source's file extension*.


[NinjaRules]: https://ninja-build.org/manual.html#_rules
