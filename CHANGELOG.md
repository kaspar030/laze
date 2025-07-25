# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.1.38] - 2025-07-17

### 🚀 Features

- Support optionally parsing laze-local.yml ([#745](https://github.com/kaspar030/laze/issues/745))

### 🐛 Bug Fixes

- Handle empty string in LAZE_BUILDERS ([#746](https://github.com/kaspar030/laze/issues/746))
- Silence jobserver output unless verbose ([#738](https://github.com/kaspar030/laze/issues/738))

### 🚜 Refactor

- env: Use std HashMap ([#743](https://github.com/kaspar030/laze/issues/743))
- Introduce EnvMap ([#739](https://github.com/kaspar030/laze/issues/739))

### 🔗  Dependencies

- deps: Bump clap_mangen from 0.2.27 to 0.2.28 ([#742](https://github.com/kaspar030/laze/issues/742))
- deps: Bump clap from 4.5.40 to 4.5.41 ([#740](https://github.com/kaspar030/laze/issues/740))
- deps: Bump clap_complete from 4.5.54 to 4.5.55 ([#741](https://github.com/kaspar030/laze/issues/741))
- deps: Bump indexmap from 2.9.0 to 2.10.0 ([#737](https://github.com/kaspar030/laze/issues/737))

## [0.1.37] - 2025-06-24

### 🚀 Features

- Add jobserver support ([#735](https://github.com/kaspar030/laze/issues/735))
- ninja: Use slash paths for ninja files ([#734](https://github.com/kaspar030/laze/issues/734))
- tasks: Implement tasks calling subtasks ([#725](https://github.com/kaspar030/laze/issues/725))

### ⚙️ Miscellaneous Tasks

- release: Add git cliff config file ([#720](https://github.com/kaspar030/laze/issues/720))

### 🔗  Dependencies

- deps: Bump subst from 0.3.7 to 0.3.8 ([#736](https://github.com/kaspar030/laze/issues/736))
- deps: Bump mimalloc from 0.1.46 to 0.1.47 ([#733](https://github.com/kaspar030/laze/issues/733))
- deps: Bump clap_complete from 4.5.52 to 4.5.54 ([#731](https://github.com/kaspar030/laze/issues/731))
- deps: Bump clap_mangen from 0.2.26 to 0.2.27 ([#729](https://github.com/kaspar030/laze/issues/729))
- deps: Bump clap from 4.5.39 to 4.5.40 ([#730](https://github.com/kaspar030/laze/issues/730))
- deps: Bump camino from 1.1.9 to 1.1.10 ([#727](https://github.com/kaspar030/laze/issues/727))
- deps: Bump clap_complete from 4.5.51 to 4.5.52 ([#726](https://github.com/kaspar030/laze/issues/726))
- deps: Bump clap from 4.5.38 to 4.5.39 ([#724](https://github.com/kaspar030/laze/issues/724))
- deps: Bump clap_complete from 4.5.50 to 4.5.51 ([#723](https://github.com/kaspar030/laze/issues/723))

## [0.1.36] - 2025-05-19

### 🚀 Features

- Create `CACHEDIR.TAG` file in build directory ([#710](https://github.com/kaspar030/laze/issues/710))
- tasks: Pass extra task args as shell args ([#702](https://github.com/kaspar030/laze/issues/702))

### ⌨️  User Interface

- Only show "laze: executing task ..." in verbose mode ([#716](https://github.com/kaspar030/laze/issues/716))

### 📚 Documentation

- Create PR links in CHANGELOG.md ([#712](https://github.com/kaspar030/laze/issues/712))

### ⚙️ Miscellaneous Tasks

- Switch to self-hosted workers ([#709](https://github.com/kaspar030/laze/issues/709))
- Bump sccache-action ([#707](https://github.com/kaspar030/laze/issues/707))
- Bump all deps ([#701](https://github.com/kaspar030/laze/issues/701))

### 🔗  Dependencies

- deps: Bump signal-hook from 0.3.17 to 0.3.18 ([#715](https://github.com/kaspar030/laze/issues/715))
- deps: Bump clap_complete from 4.5.49 to 4.5.50 ([#714](https://github.com/kaspar030/laze/issues/714))
- deps: Bump rust-embed from 8.7.0 to 8.7.1 ([#713](https://github.com/kaspar030/laze/issues/713))
- deps: Bump clap_complete from 4.5.47 to 4.5.48 ([#711](https://github.com/kaspar030/laze/issues/711))
- deps: Bump clap from 4.5.36 to 4.5.37 ([#706](https://github.com/kaspar030/laze/issues/706))
- deps: Bump shellexpand from 3.1.0 to 3.1.1 ([#704](https://github.com/kaspar030/laze/issues/704))
- deps: Bump anyhow from 1.0.97 to 1.0.98 ([#703](https://github.com/kaspar030/laze/issues/703))
- deps: Bump mimalloc from 0.1.45 to 0.1.46 ([#699](https://github.com/kaspar030/laze/issues/699))
- deps: Bump rust-embed from 8.6.0 to 8.7.0 ([#700](https://github.com/kaspar030/laze/issues/700))
- deps: Bump indexmap from 2.8.0 to 2.9.0 ([#698](https://github.com/kaspar030/laze/issues/698))
- deps: Bump mimalloc from 0.1.44 to 0.1.45 ([#697](https://github.com/kaspar030/laze/issues/697))
- deps: Bump clap from 4.5.34 to 4.5.35 ([#696](https://github.com/kaspar030/laze/issues/696))

## [0.1.35] - 2025-03-28

### Added

- feat: make output sharing optional ([#687](https://github.com/kaspar030/laze/pull/687))
- feat: add dependency resolution verbose output
- feat: track why a module was disabled, improve error msgs
- feat: expose errors resolving module providers ([#694](https://github.com/kaspar030/laze/pull/694))
- feat: improve "conflicted by" error message ([#693](https://github.com/kaspar030/laze/pull/693))

### Fixed

- fix(rule): pass through rule variables that are not in env ([#691](https://github.com/kaspar030/laze/pull/691))
- fix: pass git commit and url to `GIT_PATCH` rule ([#690](https://github.com/kaspar030/laze/pull/690))

## [0.1.34] - 2025-03-24

### Fixed

- fix(env): fix state reset with escaped expressions ([#682](https://github.com/kaspar030/laze/pull/682))

## [0.1.33] - 2025-02-24

### Added

- feat: allow setting a workdir for tasks using `workdir` (#652, #658)
- feat: initial support for Windows (#653, #648, #645)

### Fixed

- fix: use "directory" consistently across docs (drop uses of "folder") ([#655](https://github.com/kaspar030/laze/pull/655))

## [0.1.32] - 2025-02-18

### Fixed

- fix: don't leak empty variables in tasks ([#647](https://github.com/kaspar030/laze/pull/647))

## [0.1.31] - 2025-02-17

### Added

- feat: context `provides`/`provides_unique` ([#639](https://github.com/kaspar030/laze/pull/639))
- feat: provide an env variable `contexts` similar to `modules` ([#636](https://github.com/kaspar030/laze/pull/636))

## [0.1.30] - 2025-02-08

### Added

- feat: implement `includes` (`#include <file>` but for laze build files)
- feat: allow `if: - then` dependencies also for `select:`
- feat: rules `export` shell exports

## [0.1.29] - 2024-12-20

### Added

- feat(cli): implement dynamic shell completions (autocomplete builders, apps, modules)

### Fixed

- fix(cli): don't treat comma in '--define' as separator ([#590](https://github.com/kaspar030/laze/pull/590))

### Internal

- build(deps): bump clap_complete from 4.5.38 to 4.5.40
- build(deps): bump semver from 1.0.23 to 1.0.24
- build(deps): bump thiserror from 2.0.6 to 2.0.8

## [0.1.28] - 2024-12-12

### Added

- allow defining tasks in modules
- implemented `task::required_modules`

### Fixed

- always update git-cache of a branch or tag `imports` source
- track `imports:` sources, trigger new download if source has changed

## [0.1.27] - 2024-12-06

### Fixed

- fix cache laze version check again, go through bytes

## [0.1.26] - 2024-12-06

### Fixed

- fix sorting of ninja build statements, preventing ninja errors
- fix `$relroot` potentially having one `..` too much ([#571](https://github.com/kaspar030/laze/pull/571))
- fix `$root` to usually contain `.`, matching documentation
- fix cache laze version check (there was a potential crash after a laze update)
- fix buildfile caching (was broken by task `export`)

### Internal

- build(deps): bump clap from 4.5.21 to 4.5.22
- build(deps): bump anyhow from 1.0.93 to 1.0.94
- build(deps): bump indexmap from 2.6.0 to 2.7.0
- build(deps): bump pathdiff from 0.2.2 to 0.2.3

## [0.1.25] - 2024-11-25

### Added

- implement continue-on-error (add `-k`/`--keep-going`)
- implement executing multiple tasks (add `-m`/`--multiple-tasks`)
- allow contexts in `conflicts`/`disables` (prepend `context::`)

### Changed

- disallow module names starting with `context::`

### Internal

- build(deps): bump anyhow from 1.0.88 to 1.0.93
- build(deps): bump clap from 4.5.17 to 4.5.21
- build(deps): bump clap_complete from 4.5.26 to 4.5.38
- build(deps): bump clap_mangen from 0.2.23 to 0.2.24
- build(deps): bump derive_builder from 0.20.1 to 0.20.2
- build(deps): bump indexmap from 2.5.0 to 2.6.0
- build(deps): bump pathdiff from 0.2.1 to 0.2.2
- build(deps): bump serde from 1.0.210 to 1.0.215
- build(deps): bump serde_json from 1.0.128 to 1.0.133
- build(deps): bump uuid from 1.10.0 to 1.11.0

## [0.1.24] - 2024-09-13

### Added

- wire up `imports` from branches or tags, and default branch
- implement `imports` from local paths

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
[Unreleased]: https://github.com/kaspar030/laze/compare/0.1.38...HEAD
[0.1.38]: https://github.com/kaspar030/laze/compare/0.1.37...0.1.38
[0.1.37]: https://github.com/kaspar030/laze/compare/0.1.36...0.1.37
[0.1.36]: https://github.com/kaspar030/laze/compare/0.1.35...0.1.36
[0.1.35]: https://github.com/kaspar030/laze/compare/0.1.34...0.1.35
[0.1.34]: https://github.com/kaspar030/laze/compare/0.1.33...0.1.34
[0.1.33]: https://github.com/kaspar030/laze/compare/0.1.32...0.1.33
[0.1.32]: https://github.com/kaspar030/laze/compare/0.1.31...0.1.32
[0.1.31]: https://github.com/kaspar030/laze/compare/0.1.30...0.1.31
[0.1.30]: https://github.com/kaspar030/laze/compare/0.1.29...0.1.30
[0.1.29]: https://github.com/kaspar030/laze/compare/0.1.28...0.1.29
[0.1.28]: https://github.com/kaspar030/laze/compare/0.1.27...0.1.28
[0.1.27]: https://github.com/kaspar030/laze/compare/0.1.26...0.1.27
[0.1.26]: https://github.com/kaspar030/laze/compare/0.1.25...0.1.26
[0.1.25]: https://github.com/kaspar030/laze/compare/0.1.24...0.1.25
[0.1.24]: https://github.com/kaspar030/laze/compare/0.1.23...0.1.24
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
