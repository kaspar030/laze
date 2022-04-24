# Introduction

**laze** is a build system for C/C++ projects. It can be used in many cases
instead of Make, CMake or Meson.

It's main differentiators are:

- declarative, **easy to use** yaml-based build descriptions
- designed to **handle massively modular projects**
  - main driver: [RIOT OS][riot], which builds
    \> 100k configurations as part of its CI testing
  - powerful module system
  - first-class cross compilation
  - first-class multi-repository support
- optimized for **fast build configuration**
  - written in Rust
  - extensive caching
  - multithreaded build configuration
- optimized for **fast building**
  - automatically re-uses objects that are built identically for different
    build configurations
- actual building is done transparently by [Ninja][ninja]

This book contains a guide on how to install and use laze, and a reference for
its build description format.

## How does it look

Consider a simple application consisting of a single `hello.c` source file.

This laze file, saved to `laze-project.yml` next to `hello.c`, would build it:

```yaml
{{#include ../../examples/hello-world/laze-project.yml}}
```

The application can now be built:

    laze build

The resulting executable ends up in `build/out/host/hello/hello.elf`.

Alternatively,

    laze task run

Would run the executable, (re-)building it if necessary.

## Contributing

laze is free and open source. You can find the source code on GitHub and issues
and feature requests can be posted on the GitHub issue tracker. laze relies on
the community to fix bugs and add features: if you'd like to contribute, please
read the CONTRIBUTING guide and consider opening a pull request.

## License

laze source code is released under the [Apache 2.0 license][apache2.0].

This book has been heavily inspired by and is based on the [mdBook user guide][mdbook_user_guide],
and is thus licenced under the [Mozilla Public License v2.0][mpl2.0].

[riot]: https://github.com/RIOT-OS/RIOT
[ninja]: https://ninja-build.org/
[apache2.0]: https://github.com/kaspar030/laze/blob/master/LICENSE
[mdbook_user_guide]: https://rust-lang.github.io/mdBook/
[mpl2.0]: https://www.mozilla.org/en-US/MPL/2.0/
