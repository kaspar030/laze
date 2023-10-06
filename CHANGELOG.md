# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Changed

### Fixed

- fixed expression evaluation for tasks

### Internal

- more CHANGELOG.md automation

## [0.1.16]

### Changed

- BREAKING: allow basic expressions in variables and task commands
  This is using the `evalexpr` crate. Expressions are used make-style
  by wrapping them in `$()`, e.g., `$(1+2)`.

  This requires previous `$(echo foo)` meant to be subshells to be escaped:
  `$$(echo foo)`.

- implement task `required_vars`
  This allows task availability depend on the existence of global variables.

- cli: drop "tasks" cli subcommand, add tasks to "build"
  Instead of `laze task ... <task-name> [<task args>]`, just do
  `laze build ... <task-name> [<task args>]`.

- cli: improve "build" subcommand help message

### Fixed

- global build deps are now applied to the LINK step. That fixes build targets
  that have no created objects where the dependencies can be added to, e.g.,
  a module with 'download:' but without `sources:` / objects.

## [0.1.15]

### Fixed

- fix release / rebase mishap

## [0.1.14]

### Changed

- disallow space seperated lists for apps, builders, define, selects, disables
  Lists can still be specified as comma-seperated (e.g., `--apps foo,bar`),
  or by doing e.g., `--apps foo --apps bar`.

- MSRV bumped to 1.64

### Added

- modules: add `provides_unique` as shortcut to both provide and conflict
- download and patch rules are now module-env expanded

## [0.1.13]

### Fixed

- fix: fix not selected provided modules leaking exports

### Dependencies

- bump mimalloc from 0.1.32 to 0.1.34
- bump clap from 4.0.32 to 4.1.1
- bump clap_mangen from 0.2.6 to 0.2.7

## [0.1.12]

### Added

- modules: add "is_global_build_dep"

### Fixed

- exports from "provided" dependencies are now properly imported

### Dependencies

- bump anyhow from 1.0.66 to 1.0.68
- bump clap from 4.0.29 to 4.0.32
- bump clap_complete from 4.0.6 to 4.0.7
- bump clap_mangen from 0.2.5 to 0.2.6
- bump serde from 1.0.150 to 1.0.152
- bump serde_yaml from 0.9.14 to 0.9.16

## [0.1.11]

### Added

- cli: implement partitioning
- cli: experimental: add (hidden) commands to create shell completions and man
       pages

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

<!-- next-url -->
[Unreleased]: https://github.com/kaspar030/laze/compare/0.1.16...HEAD
[0.1.16]: https://github.com/kaspar030/laze/compare/0.1.15...0.1.16
[0.1.15]: https://github.com/kaspar030/laze/compare/0.1.14...0.1.15
[0.1.14]: https://github.com/kaspar030/laze/compare/0.1.13...0.1.14
[0.1.13]: https://github.com/kaspar030/laze/compare/0.1.12...0.1.13
[0.1.12]: https://github.com/kaspar030/laze/compare/0.1.11...0.1.12
[0.1.11]: https://github.com/kaspar030/laze/compare/0.1.10...0.1.11
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
