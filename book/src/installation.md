# Installation

There are multiple ways to install the laze CLI tool.
Choose any one of the methods below that best suit your needs.
If you are installing laze for automatic deployment, check out the [continuous integration] chapter for more examples on how to install.

[continuous integration]: ../continuous-integration.md

## Distribution packages

The easiest way to install laze is probably by using a package for your
distribution.

Available packages:

| Distribution &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp;   | Package name |
|----------------|--------------|
| Arch Linux AUR | [`laze-bin`](https://aur.archlinux.org/packages/laze-bin)   |

## Dependencies

laze requires [Ninja](https://ninja-build.org). You can [download the Ninja binary](https://github.com/ninja-build/ninja/releases) or [find it in your system's package manager](https://github.com/ninja-build/ninja/wiki/Pre-built-Ninja-packages).

## Pre-compiled binaries

Executable binaries are available for download on the [GitHub Releases page][releases].
Download the binary for your platform (Windows, macOS, or Linux) and extract
the archive. The archive contains `laze` executable.

To make it easier to run, put the path to the binary into your `PATH`.

[releases]: https://github.com/kaspar030/laze/releases

## Build from source using Rust

To build the `mdbook` executable from source, you will first need to install Rust and Cargo.
Follow the instructions on the [Rust installation page].
laze currently requires at least Rust version 1.54.

Once you have installed Rust, the following command can be used to build and install laze:

```sh
cargo install laze
```

This will automatically download laze from [crates.io], build it, and install it in Cargo's global binary directory (`~/.cargo/bin/` by default).

[rust installation page]: https://www.rust-lang.org/tools/install
[crates.io]: https://crates.io/

### Installing the latest master version

The version published to crates.io will ever so slightly be behind the version hosted on GitHub.
If you need the latest version you can build the git version of laze yourself.
Cargo makes this **_super easy_**!

```sh
cargo install --git https://github.com/kaspar030.git laze
```

Again, make sure to add the Cargo bin directory to your `PATH`.

If you are interested in making modifications to laze itself, check out the [Contributing Guide] for more information.

[contributing guide]: https://github.com/kaspar030/laze/blob/master/CONTRIBUTING.md
