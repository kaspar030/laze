#[macro_use]
extern crate anyhow;
extern crate clap;

#[macro_use]
extern crate simple_error;

#[macro_use]
extern crate derive_builder;

extern crate pathdiff;

use core::sync::atomic::AtomicBool;
use std::collections::HashSet;
use std::env;
use std::iter;
use std::path::{Path, PathBuf};
use std::thread;

#[macro_use]
extern crate serde_derive;

use anyhow::{Context as _, Error, Result};
use clap::{crate_version, Arg, Command};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use signal_hook::{consts::SIGINT, iterator::Signals};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod data;
mod download;
mod generate;
mod model;
mod nested_env;
mod ninja;
mod serde_bool_helpers;
mod utils;

use model::{Context, ContextBag, Dependency, Module, Rule, Task};

use generate::{get_ninja_build_file, BuildInfo, GenerateMode, GeneratorBuilder, Selector};
use nested_env::{Env, MergeOption};
use ninja::NinjaCmdBuilder;

// impl fmt::Display for Context {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         for context in &self.contexts {
//             let parent_name = match context.parent {
//                 Some(index) => self.contexts[index].name.clone(),
//                 None => "none".to_string(),
//             };

//             println!("context: {} parent: {}", context.name, parent_name);
//         }
//     }
// }

struct Build<'a> {
    bag: &'a ContextBag,
    binary: Module,
    builder: &'a Context,
    build_context: Context,
    //modules: IndexMap<&'a String, &'a Module>,
}

impl<'a: 'b, 'b> Build<'b> {
    fn new(
        binary: &'a Module,
        builder: &'a Context,
        contexts: &'a ContextBag,
        cli_selects: Option<&Vec<Dependency<String>>>,
    ) -> Build<'b> {
        let build_context = Context::new_build_context(builder.name.clone(), builder);

        // TODO: opt: see if Cow improves performance
        let mut binary = binary.clone();

        // add all "select:" from contexts to this build's binary.
        // the selects will be appended, making it possible to override contexts)
        // selects from the binary.
        let context_selects = build_context.collect_selected_modules(contexts);
        if !context_selects.is_empty() {
            binary.selects.extend(context_selects);
        }

        // add all selects from CLI
        if let Some(selects) = cli_selects {
            binary.selects.extend(selects.iter().cloned());
        }

        let mut build = Build {
            bag: contexts,
            binary,
            builder,
            build_context: Context::new_build_context(builder.name.clone(), builder),
        };

        /* fixup name to "$builder_name:$binary_name" */
        build.build_context.name.push(':');
        build.build_context.name.push_str(&build.binary.name);

        /* collect environment from builder */
        let mut build_env;
        if let Some(builder_env) = &builder.env {
            build_env = builder_env.clone();
        } else {
            build_env = Env::new();
        }

        // insert "builder" variable
        build_env.insert(
            "builder".to_string(),
            nested_env::EnvKey::Single(builder.name.clone()),
        );
        // add "app" variable
        build_env.insert(
            "app".to_string(),
            nested_env::EnvKey::Single(build.binary.name.clone()),
        );

        build.build_context.env = Some(build_env);

        build
    }

    fn resolve_module_deep<'m, 's: 'm>(
        &'s self,
        module: &'m Module,
        module_set: &mut IndexMap<&'m String, &'m Module>,
        if_then_deps: &mut IndexMap<String, Vec<Dependency<String>>>,
        disabled_modules: &mut IndexSet<String>,
    ) -> Result<(), Error> {
        let prev_len = module_set.len();
        let if_then_deps_prev_len = if_then_deps.len();
        let disabled_modules_prev_len = disabled_modules.len();

        module_set.insert(&module.name, module);
        if let Some(conflicts) = &module.conflicts {
            disabled_modules.extend(conflicts.iter().cloned())
        }

        let mut late_if_then_deps = Vec::new();
        if let Some(deps) = if_then_deps.get(&module.name) {
            late_if_then_deps.extend(deps.iter().cloned());
        }

        for dep in module.selects.iter().chain(late_if_then_deps.iter()) {
            let (dep_name, optional) = match dep {
                Dependency::Hard(name) => (name, false),
                Dependency::Soft(name) => (name, true),
                Dependency::IfThenHard(other, name) => {
                    if module_set.contains_key(other) {
                        (name, false)
                    } else {
                        if_then_deps
                            .entry(other.clone())
                            .or_insert_with(Vec::new)
                            .push(Dependency::Hard(name.clone()));
                        continue;
                    }
                }
                Dependency::IfThenSoft(other, name) => {
                    if module_set.contains_key(other) {
                        (name, true)
                    } else {
                        if_then_deps
                            .entry(other.clone())
                            .or_insert_with(Vec::new)
                            .push(Dependency::Soft(name.clone()));

                        continue;
                    }
                }
            };

            if module_set.contains_key(dep_name) {
                continue;
            }

            if disabled_modules.contains(dep_name) {
                if !optional {
                    module_set.truncate(prev_len);
                    if_then_deps.truncate(if_then_deps_prev_len);
                    disabled_modules.truncate(disabled_modules_prev_len);

                    bail!(
                        "binary {} for builder {}: {} depends on disabled/conflicted module {}",
                        self.binary.name,
                        self.builder.name,
                        module.name,
                        dep_name
                    );
                } else {
                    continue;
                }
            }

            let (_context, module) = match self.build_context.resolve_module(dep_name, self.bag) {
                Some(x) => x,
                None => {
                    if optional {
                        continue;
                    } else {
                        module_set.truncate(prev_len);
                        if_then_deps.truncate(if_then_deps_prev_len);
                        disabled_modules.truncate(disabled_modules_prev_len);
                        bail!(
                            "binary {} for builder {}: {} depends on unavailable module {}",
                            self.binary.name,
                            self.builder.name,
                            module.name,
                            dep_name
                        );
                    }
                }
            };

            if let Err(x) =
                self.resolve_module_deep(module, module_set, if_then_deps, disabled_modules)
            {
                if !optional {
                    module_set.truncate(prev_len);
                    if_then_deps.truncate(if_then_deps_prev_len);
                    disabled_modules.truncate(disabled_modules_prev_len);
                    return Err(x);
                }
            }
        }
        Ok(())
    }

    fn resolve_selects(
        &self,
        disabled_modules: &mut IndexSet<String>,
    ) -> Result<IndexMap<&String, &Module>, Error> {
        let mut modules = IndexMap::new();
        let mut if_then_deps = IndexMap::new();

        if let Err(x) = self.resolve_module_deep(
            &self.binary,
            &mut modules,
            &mut if_then_deps,
            disabled_modules,
        ) {
            return Err(x);
        }

        Ok(modules)
    }
}

fn determine_project_root(start: &PathBuf) -> Result<(PathBuf, PathBuf)> {
    let mut cwd = start.clone();

    loop {
        let mut tmp = cwd.clone();
        tmp.push("laze-project.yml");
        if tmp.exists() {
            return Ok((cwd, PathBuf::from("laze-project.yml")));
        }
        cwd = match cwd.parent() {
            Some(p) => PathBuf::from(p),
            None => return Err(anyhow!("cannot find laze-project.yml")),
        }
    }
}

fn ninja_run(
    ninja_buildfile: &Path,
    verbose: bool,
    targets: Option<Vec<PathBuf>>,
    jobs: Option<usize>,
) -> Result<i32, Error> {
    let mut ninja_cmd = NinjaCmdBuilder::default();

    ninja_cmd
        .verbose(verbose)
        .build_file(ninja_buildfile.to_str().unwrap())
        .targets(targets);

    if let Some(jobs) = jobs {
        ninja_cmd.jobs(jobs);
    }

    let ninja_exit = ninja_cmd.build().unwrap().run()?;

    match ninja_exit.code() {
        Some(code) => match code {
            0 => Ok(code),
            _ => Err(anyhow!("ninja exited with code {}", code)),
        },
        None => Err(anyhow!("ninja probably killed by signal")),
    }
}

fn main() {
    let result = try_main();
    match result {
        Err(e) => {
            eprintln!("laze: error: {:#}", e);
            std::process::exit(1);
        }
        Ok(code) => std::process::exit(code),
    };
}

pub static IGNORE_SIGINT: AtomicBool = AtomicBool::new(false);

fn clap() -> clap::Command<'static> {
    Command::new("laze in rust")
        .version(crate_version!())
        .author("Kaspar Schleiser <kaspar@schleiser.de>")
        .about("Build a lot, fast")
        .infer_subcommands(true)
        .arg(
            Arg::new("chdir")
                .short('C')
                .long("chdir")
                .help("change working directory before doing anything else")
                .global(true)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new("global")
                .short('g')
                .long("global")
                .help("global mode")
                .global(true)
                .required(false),
        )
        .subcommand(
            Command::new("build")
                .about("generate build files and build")
                .arg(
                    Arg::new("build-dir")
                        .short('B')
                        .long("build-dir")
                        .takes_value(true)
                        .value_name("DIR")
                        .default_value("build")
                        .help("specify build dir (relative to project root)"),
                )
                .arg(
                    Arg::new("generate-only")
                        .short('G')
                        .long("generate-only")
                        .help("generate build files only, don't start build")
                        .required(false),
                )
                .arg(
                    Arg::new("builders")
                        .short('b')
                        .long("builders")
                        .help("builders to configure")
                        .required(false)
                        .takes_value(true)
                        .multiple_values(true)
                        .use_value_delimiter(true)
                        .require_value_delimiter(true)
                        .env("LAZE_BUILDERS"),
                )
                .arg(
                    Arg::new("apps")
                        .short('a')
                        .long("apps")
                        .help("apps to configure")
                        .required(false)
                        .takes_value(true)
                        .multiple_values(true)
                        .use_value_delimiter(true)
                        .require_value_delimiter(true)
                        .env("LAZE_APPS"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("be verbose (e.g., show command lines)")
                        .multiple_occurrences(true),
                )
                .arg(
                    Arg::new("jobs")
                        .short('j')
                        .long("jobs")
                        .help("how many compile jobs to run in parallel")
                        .takes_value(true)
                        .validator(|val| val.parse::<usize>()),
                )
                .arg(
                    Arg::new("select")
                        .short('s')
                        .long("select")
                        .alias("enable")
                        .help("extra modules to select/enable")
                        .required(false)
                        .takes_value(true)
                        .multiple_values(true)
                        .use_value_delimiter(true)
                        .require_value_delimiter(true)
                        .env("LAZE_SELECT"),
                )
                .arg(
                    Arg::new("disable")
                        .short('d')
                        .long("disable")
                        .help("disable modules")
                        .required(false)
                        .takes_value(true)
                        .multiple_values(true)
                        .use_value_delimiter(true)
                        .require_value_delimiter(true)
                        .env("LAZE_DISABLE"),
                )
                .arg(
                    Arg::new("define")
                        .short('D')
                        .long("define")
                        .help("set/override variable")
                        .required(false)
                        .takes_value(true)
                        .multiple_occurrences(true)
                        //                        .number_of_values(1)
                        .env("LAZE_DEFINE"),
                ),
        )
        .subcommand(
            Command::new("task")
                .about("run builder specific task")
                .override_usage("laze task [FLAGS] [OPTIONS] <TASK> [ARGS]...")
                .allow_external_subcommands(true)
                .subcommand_required(true)
                .arg(
                    Arg::new("build-dir")
                        .short('B')
                        .long("build-dir")
                        .takes_value(true)
                        .value_name("DIR")
                        .default_value("build")
                        .help("specify build dir (relative to project root)"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("be verbose (e.g., show command lines)")
                        .multiple_occurrences(true),
                )
                .arg(
                    Arg::new("jobs")
                        .short('j')
                        .long("jobs")
                        .help("how many compile jobs to run in parallel")
                        .takes_value(true)
                        .validator(|val| val.parse::<usize>()),
                )
                .arg(
                    Arg::new("builder")
                        .short('b')
                        .long("builder")
                        .help("builder to run task for")
                        .required(false)
                        .takes_value(true)
                        .env("LAZE_BUILDERS"),
                )
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .help("application target to run task for")
                        .required(false)
                        .takes_value(true)
                        .env("LAZE_APPS"),
                )
                .arg(
                    Arg::new("select")
                        .short('s')
                        .long("select")
                        .alias("enable")
                        .help("extra modules to select/enable")
                        .required(false)
                        .takes_value(true)
                        .multiple_values(true)
                        .use_value_delimiter(true)
                        .require_value_delimiter(true)
                        .env("LAZE_SELECT"),
                )
                .arg(
                    Arg::new("disable")
                        .short('d')
                        .long("disable")
                        .help("disable modules")
                        .required(false)
                        .takes_value(true)
                        .multiple_values(true)
                        .use_value_delimiter(true)
                        .require_value_delimiter(true)
                        .env("LAZE_DISABLE"),
                )
                .arg(
                    Arg::new("define")
                        .short('D')
                        .long("define")
                        .help("set/override variable")
                        .required(false)
                        .takes_value(true)
                        .multiple_occurrences(true)
                        .env("LAZE_DEFINE"),
                ),
        )
        .subcommand(
            Command::new("clean")
                .about("clean current configuration")
                .arg(
                    Arg::new("build-dir")
                        .short('B')
                        .long("build-dir")
                        .takes_value(true)
                        .value_name("DIR")
                        .default_value("build")
                        .help("specify build dir (relative to project root)"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("be verbose (e.g., show command lines)")
                        .multiple_occurrences(true),
                )
                .arg(
                    Arg::new("unused").short('u').long("unused").help(
                        "clean built files that are not produced by the current configuration",
                    ),
                ),
        )
}

fn try_main() -> Result<i32> {
    let mut signals = Signals::new(&[SIGINT])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            if sig == SIGINT && !IGNORE_SIGINT.load(std::sync::atomic::Ordering::SeqCst) {
                std::process::exit(130);
            }
        }
    });

    let matches = clap().get_matches();

    if let Some(dir) = matches.value_of("chdir") {
        env::set_current_dir(dir).context(format!("cannot change to directory \"{}\"", dir))?;
    }

    let cwd = env::current_dir()?;

    let (project_root, project_file) = determine_project_root(&cwd)?;
    let start_relpath = pathdiff::diff_paths(&cwd, &project_root).unwrap();

    println!(
        "laze: project root: {} relpath: {} project_file: {}",
        project_root.display(),
        start_relpath.display(),
        project_file.display()
    );

    let global = matches.is_present("global");
    env::set_current_dir(&project_root)
        .context(format!("cannot change to \"{}\"", &project_root.display()))?;

    match matches.subcommand() {
        Some(("build", build_matches)) => {
            let verbose = build_matches.occurrences_of("verbose");
            let build_dir = PathBuf::from(build_matches.value_of("build-dir").unwrap());

            // collect builder names from args
            let builders = match build_matches.values_of("builders") {
                Some(values) => {
                    Selector::Some(values.map(String::from).collect::<IndexSet<String>>())
                }
                None => Selector::All,
            };

            // collect app names from args
            let apps = match build_matches.values_of("apps") {
                Some(values) => {
                    Selector::Some(values.map(String::from).collect::<IndexSet<String>>())
                }
                None => Selector::All,
            };

            let jobs = build_matches
                .value_of("jobs")
                .map_or(None, |val| Some(val.parse::<usize>().unwrap()));

            println!("building {} for {}", &apps, &builders);

            // collect CLI selected modules
            let select = build_matches.values_of_lossy("select");
            // convert CLI --select strings to Vec<Dependency>
            let select = select.map(|mut vec| {
                vec.drain(..)
                    .map(|dep_name| crate::data::dependency_from_string(&dep_name))
                    .collect_vec()
            });

            let disable = build_matches.values_of_lossy("disable");

            // collect CLI env overrides
            let cli_env = if build_matches.occurrences_of("define") > 0 {
                let mut env = Env::new();

                for assignment in build_matches.values_of("define").unwrap() {
                    env = nested_env::assign_from_string(env, assignment)?;
                }

                Some(env)
            } else {
                None
            };

            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath),
            };

            let generator = GeneratorBuilder::default()
                .project_root(project_root)
                .project_file(project_file)
                .build_dir(build_dir.clone())
                .mode(mode.clone())
                .builders(builders.clone())
                .apps(apps.clone())
                .select(select)
                .disable(disable)
                .cli_env(cli_env)
                .build()
                .unwrap();

            // arguments parsed, launch generation of ninja file(s)
            let builds = generator.execute()?;

            // generation of ninja build file complete.
            // exit here if requested.
            if build_matches.is_present("generate-only") {
                return Ok(0);
            }

            // build ninja target arguments, if necessary
            let targets: Option<Vec<PathBuf>> = if let Selector::All = builders {
                if let Selector::All = apps {
                    None
                } else {
                    None
                }
            } else {
                Some(
                    builds
                        .build_infos
                        .iter()
                        .filter_map(|(builder, app, build_info)| {
                            if builders.selects(builder) && apps.selects(app) {
                                Some(build_info.out.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            };

            let ninja_build_file = get_ninja_build_file(&build_dir, &mode);
            ninja_run(ninja_build_file.as_path(), verbose > 0, targets, jobs)?;
        }
        Some(("task", task_matches)) => {
            let verbose = task_matches.occurrences_of("verbose");
            let build_dir = Path::new(task_matches.value_of("build-dir").unwrap());

            let builder = task_matches.value_of("builder");
            let app = task_matches.value_of("app");

            let jobs = task_matches
                .value_of("jobs")
                .map_or(None, |val| Some(val.parse::<usize>().unwrap()));

            // collect CLI selected modules
            let select = task_matches.values_of_lossy("select");
            // convert CLI --select strings to Vec<Dependency>
            let select = select.map(|mut vec| {
                vec.drain(..)
                    .map(|dep_name| crate::data::dependency_from_string(&dep_name))
                    .collect_vec()
            });

            let disable = task_matches.values_of_lossy("disable");

            let (task, args) = match task_matches.subcommand() {
                Some((name, matches)) => {
                    let args = matches.values_of("").map(|v| v.collect());
                    (name, args)
                }
                _ => unreachable!(),
            };

            // collect builder names from args
            let builders = match builder {
                Some(builder) => {
                    Selector::Some(iter::once(builder.into()).collect::<IndexSet<String>>())
                }
                None => Selector::All,
            };

            // collect app names from args
            let apps = match app {
                Some(app) => Selector::Some(iter::once(app.into()).collect::<IndexSet<String>>()),
                None => Selector::All,
            };

            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath),
            };

            // collect CLI env overrides
            let cli_env = if task_matches.occurrences_of("define") > 0 {
                let mut env = Env::new();

                for assignment in task_matches.values_of("define").unwrap() {
                    env = nested_env::assign_from_string(env, assignment)?;
                }

                Some(env)
            } else {
                None
            };

            println!("building {} for {}", &apps, &builders);

            // arguments parsed, launch generation of ninja file(s)
            let generator = GeneratorBuilder::default()
                .project_root(&project_root)
                .project_file(project_file)
                .build_dir(build_dir)
                .mode(mode.clone())
                .builders(builders.clone())
                .apps(apps.clone())
                .select(select)
                .disable(disable)
                .cli_env(cli_env)
                .build()
                .unwrap();

            let builds = generator.execute()?;

            let builds: Vec<&(String, String, BuildInfo)> = builds
                .build_infos
                .iter()
                .filter(|(builder, app, build_info)| {
                    builders.selects(builder)
                        && apps.selects(app)
                        && build_info.tasks.contains_key(task)
                })
                .collect();

            if builds.len() > 1 {
                eprintln!("laze: multiple task targets found:");
                for (builder, bin, _build_info) in builds {
                    eprintln!("{} {}", builder, bin);
                }

                // TODO: allow running tasks for multiple targets
                return Err(anyhow!("please specify one of these."));
            }

            if builds.is_empty() {
                return Err(anyhow!("no matching target for task \"{}\" found.", task));
            }

            let build = builds[0];
            let targets = Some(vec![build.2.out.clone()]);

            let task_name = task;
            let task = build.2.tasks.get(task).unwrap();

            if task.build_app() {
                let ninja_build_file = get_ninja_build_file(build_dir, &mode);
                if ninja_run(ninja_build_file.as_path(), verbose > 0, targets, jobs)? != 0 {
                    return Err(anyhow!("build error"));
                };
            }

            println!(
                "laze: executing task {} for builder {} bin {}",
                task_name, build.0, build.1,
            );

            task.execute(project_root.as_ref(), args, verbose)?;
        }
        Some(("clean", clean_matches)) => {
            let verbose = clean_matches.occurrences_of("verbose");
            let unused = clean_matches.is_present("unused");
            let build_dir = PathBuf::from(clean_matches.value_of("build-dir").unwrap());
            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath),
            };
            let ninja_build_file = get_ninja_build_file(&build_dir, &mode);
            let tool = match unused {
                true => "cleandead",
                false => "clean",
            };
            let clean_target: Option<Vec<PathBuf>> = Some(vec!["-t".into(), tool.into()]);
            ninja_run(ninja_build_file.as_path(), verbose > 0, clean_target, None)?;
        }
        _ => {}
    };

    Ok(0)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_clap() {
        crate::clap().debug_assert();
    }
}
