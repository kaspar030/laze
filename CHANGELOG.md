# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

### Added

- cli: implement partitioning

## [0.1.10]

### Fixed

- cli: fix `-C <dir>`
- cli: change name to just `laze` (from `laze in Rust`)

## [0.1.9]

### Added

- modules: add "provides"
- cli: allow `--build-dir`, `--jobs`, `--compile-commands`, `--global`
  to be set via environment
- cli: wire up creation of compile_commands.json

### Fixed

- make CLI selects have precedence over app/context selects

## [0.1.8]

### Changed

- "app:" -> "apps:"
- "module:" -> "modules:"
- "context:" -> "contexts:"
- "builder:" -> "builders:"
- "contexts.rule" -> "context.rules"
- "context.disable" -> "context.disables"

### Added

- mdBook manual

## [0.1.7]

### Added

- add `-j<N>` to build/task, pass to ninja

### Fixed

- fix release workflow

## [0.1.6]

### Added

- "import" from git
- "import" from assets bundled with laze (e.g., "import: laze: defaults")
- custom module builds (`build:`)
- allow "--enable" as alias for "--select"
- add laze binary build id to cache hash
- allow overriding default download dir with "dldir" option
- expand variables in "sources" and "srcdir"
- pass "description" to Ninja rules
- "clean" cli command
- switch to mimalloc
- providing binary releases on Github
- set "project-root" variable
- allow specifying custom "outfile"
- this CHANGELOG.md

### Fixed

- apply early environment (e.g., `$root`) to task commands
- don't panic on duplicate context definition, show involved files
- fix "if_then" dependency order
- fix build dependencies
- take "app" context into account

## [0.1.5] - 2021-09-21

### Added

-

## [0.1.4] - 2021-07-07

## [0.1.3] - 2021-07-05

## [0.1.2] - 2021-03-08

## [0.1.1] - 2021-01-07

[unreleased]: https://github.com/kaspar030/laze/compare/0.1.10...HEAD
[0.1.10]: https://github.com/kaspar030/laze/compare/0.1.9...0.1.10
[0.1.9]: https://github.com/kaspar030/laze/compare/0.1.8...0.1.9
[0.1.8]: https://github.com/kaspar030/laze/compare/v0.1.7...0.1.8
[0.1.7]: https://github.com/kaspar030/laze/compare/v0.1.6...0.1.7
[0.1.6]: https://github.com/kaspar030/laze/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/kaspar030/laze/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/kaspar030/laze/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/kaspar030/laze/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/kaspar030/laze/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/kaspar030/laze/releases/tag/v0.1.1
