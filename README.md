[![CI](https://github.com/kaspar030/laze/actions/workflows/tests.yml/badge.svg)](https://github.com/kaspar030/laze/actions/workflows/tests.yml)
[![Dependency Status](https://deps.rs/repo/github/kaspar030/laze/status.svg)](https://deps.rs/repo/github/kaspar030/laze)
[![Coverage Status](https://coveralls.io/repos/github/kaspar030/laze/badge.svg)](https://coveralls.io/github/kaspar030/laze)
[![Packaging status](https://repology.org/badge/tiny-repos/laze.svg)](https://repology.org/project/laze/versions)
[![latest packaged version(s)](https://repology.org/badge/latest-versions/laze.svg)](https://repology.org/project/laze/versions)
![MSRV](https://img.shields.io/crates/msrv/laze)

<img src="./book/images/logo_col_bg.svg">

# Introduction

Welcome to laze, powerful meta build system designed to handle large build
matrices of highly modular projects.


# Installation

Install the ninja build tool using your distro's package manager, then install
laze using cargo:

    $ cargo install laze


# Getting started

    $ laze -C examples/hello-world build run


# Documentation

Please take a look at the
[manual](https://kaspar030.github.io/laze/dev/index.html). It is still
incomplete, but being worked on.

# License

laze is licensed under the terms of the Apache License (Version 2.0).
