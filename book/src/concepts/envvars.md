### Variables and Environments

Apps/modules and contexts/builders can have _variables_. Variables are combined
in _environments_.

Laze will collect all variables of a build's builder and its ancestor contexts
into that build's _global environment_.

Each app/module can have _local_, _global_ and _exported_ variables.
All _global_ app/module variables get merged into a builds global environment.

An app/module's _local environment_ consist of the build's global environment,
the app/module's local variables and its own and all it's dependencies' exported
environments, transitively.

An app/module's _exported environment_ consists of its own exported variables
and all it's dependencies' exported environments, transitively.

Confusing? Maybe. It is totally possible to only use global variables, and for
smaller projects, that won't hurt too much.

But imagine some module needs to be compiled with some `CFLAGS` define that no
other module cares about. If set globally, all files would need to be
recompiled whenever that define changes. If set as a local variable, only that
module's files need recompilation. But what if that define is evaluated in an
API defining header that might be included by another module (a dependee of our
first module)? In that case, the define must also be exported, so all dependees
that make use of the header get the define.
