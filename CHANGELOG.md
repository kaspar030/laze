# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.1.23] - 2024-09-07

## [0.1.22] - 2024-09-07

### Added

- integrate `git-cache-rs`
- introduce `var_options::from`
- Add `context::buildable`, allowing a context to become a builder.

## [0.1.21] - 2024-02-13

### Added

- Allow overriding the `dldir` for (`download`, `cmd`) imports

### Changed

- Make yaml parsing strict (disallow unexpected fields)
- Introduce "meta" field on contexts, modules, rules, tasks and at file level.
  This field will just be ignored, allowing for extra information in laze build
  files now that the yaml parsing is strict.
- Strip binary by default

### Fixes

- Fix CHANGELOG.md links, update `cargo release` pre-release-replacements

### Dependencies

- Bump clap from 4.4.13 to 4.4.18
- Bump clap_complete from 4.4.6 to 4.4.10
- Bump clap_mangen from 0.2.16 to 0.2.19
- Bump container.yml versions
- Bump derive_builder from 0.12.0 to 0.13.0
- Bump indexmap from 2.1.0 to 2.2.2
- Bump itertools from 0.12.0 to 0.12.1
- Bump rayon from 1.8.0 to 1.8.1
- Bump serde from 1.0.194 to 1.0.196
- Bump serde_json from 1.0.111 to 1.0.113
- Bump serde_yaml from 0.9.30 to 0.9.31
- Bump treestate from 0.1.0 to 0.1.1
- Bump uuid from 1.6.1 to 1.7.0

### Internal

- import: Factor out download.rs
- nested_env: Improve error message

## [0.1.20] - 2024-01-08

### Added

- `POST_LINK` rule: added a way to post process app `.elf` files

### Fixed

- `laze new` now drops the `.in` extension from rendered templates

### Dependencies

- bump clap from 4.4.12 to 4.4.13

### Internal

- refactored `nested_env::Env` to be a distinct struct. This brought a nice
  performance gain (+~5% for RIOT use case).

## [0.1.19] - 2024-01-05

### Added

- implement `laze build --info-export <file.json> ...`, exporting some insights
  (modules \& dependencies)
- implement `laze new`, a way to generate laze projects from templates

### Changed

- cli: make "--verbose" a global flag

### Fixed

- `${relpath}` now contains '.' if otherwise emtpy in more cases
- correctly find applications in local mode when in project root
- use buffered IO for cache. Speeds up cache read/write dramatically.
- if the srcdir of a module is equal or a descendent of a dependency, create
  a ninja phony rule. Prevents ninja from complaining about non-existant files.

### Internal

- set up continuous benchmarking using bencher
- prevent `fixup!` commits from getting merged
- build(deps): bump anyhow from 1.0.75 to 1.0.79
- build(deps): bump clap from 4.4.11 to 4.4.12
- build(deps): bump clap_complete from 4.4.4 to 4.4.6
- build(deps): bump clap_mangen from 0.2.15 to 0.2.16
- build(deps): bump rust-embed from 8.1.0 to 8.2.0
- build(deps): bump semver from 1.0.20 to 1.0.21
- build(deps): bump serde from 1.0.193 to 1.0.194
- build(deps): bump serde_yaml from 0.9.27 to 0.9.30

## [0.1.18] - 2023-12-18

### Added

- add file-level `laze_required_version`
  If specified, laze checks if its own version is at least`laze_required_version`.

- add `task::exports`
  This allows laze variables to be exported to the envirenment of tasks:

  ```yaml
    context:
    - name: foo
      env:
        some_variable: some_value
      tasks:
        some_task:
          exports:
          - some_variable
        cmd:
        # some_varialble=some_value is will be set in the _shell environment_
        - some_command
    ```

### Internal

- extended test suite, now allows to test for exit codes and stdout/stderr patterns.
  Mostly, this allows to test errors.

- dependencies:
  - bump clap from 4.4.6 to 4.4.11
  - bump clap_complete from 4.4.3 to 4.4.4
  - bump clap_mangen from 0.2.14 to 0.2.15
  - bump evalexpr from 11.1.0 to 11.3.0
  - bump indexmap from 2.0.2 to 2.1.0
  - bump itertools from 0.11.0 to 0.12.0
  - bump rust-embed from 8.0.0 to 8.1.0
  - bump serde from 1.0.188 to 1.0.193
  - bump serde_yaml from 0.9.25 to 0.9.27
  - bump url from 2.4.1 to 2.5.0
  - bump uuid from 1.4.1 to 1.6.1

## [0.1.17] - 2023-10-06

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
[Unreleased]: https://github.com/kaspar030/laze/compare/0.1.23...HEAD
[0.1.23]: https://github.com/kaspar030/laze/compare/0.1.22...0.1.23
[0.1.22]: https://github.com/kaspar030/laze/compare/0.1.21...0.1.22
[0.1.21]: https://github.com/kaspar030/laze/compare/0.1.20...0.1.21
[0.1.20]: https://github.com/kaspar030/laze/compare/0.1.19...0.1.20
[0.1.19]: https://github.com/kaspar030/laze/compare/0.1.18...0.1.19
[0.1.18]: https://github.com/kaspar030/laze/compare/0.1.17...0.1.18
[0.1.17]: https://github.com/kaspar030/laze/compare/0.1.16...0.1.17
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
