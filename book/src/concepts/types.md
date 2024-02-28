### Builds, Apps, Builders, Contexts, Modules, Tasks

Conceptionally laze builds **apps** for **builders**.
Each app/builder combination that laze configures is called a **build**.

An **app** represents a binary, and a **builder** would be e.g., the toolchain
configuration for the host system.

Builders are **contexts**. All contexts (and thus builders) have a parent,
up to the root context which is called "default". A builder is just a context
that apps can be built for. Any context with `buildable: true` is a builder.

A context could be "linux", or "ubuntu" with parent context "linux". A builder
could then be "amd64" with parent context "ubuntu". Builders can have other
builders as parent context.

Contexts (and thus builders) _inherit_ modules and variables from their parents.

Apps can depend on **modules**, which can have dependencies themselves.
Technically, an app is just a special kind of module that can be built for a
builder.

Apps and modules consist of zero or more source files.

**Tasks** are operations that can be run on binaries of a build configuration.
They can be used to execute a compiled binary, run it through a debugger,
install some files, update/reset an embedded target, ...

**Tasks** are executed by laze (not ninja), after building possible dependencies.
