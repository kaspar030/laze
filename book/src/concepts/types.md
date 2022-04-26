### Builds, Apps, Builders, Contexts, Modules

Conceptionally laze builds **apps** for **builders**.
Each app/builder combination that laze configures is called a **build**.

An **app** represents a binary, and a **builder** would be e.g., the toolchain
configuration for the host system.

Builders are **contexts**. All contexts (and thus builders) have a parent,
up to the root context which is called "default". A builder is just a context
that apps can be built for.

A context could be "linux", or "ubuntu" with parent context "linux". A builder
could then be "amd64" with parent context "ubuntu". Builders can have other
builders as parent context.

Apps can depend on **modules**, which can have dependencies themselves.
Technically, an app is just a special kind of module that can be built for a
builder.

Apps and modules consist of zero or more source files.