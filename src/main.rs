#[macro_use]
extern crate anyhow;
extern crate clap;

#[macro_use]
extern crate simple_error;

#[macro_use]
extern crate derive_builder;

extern crate pathdiff;

use core::sync::atomic::AtomicBool;

use std::env;
use std::iter;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::str;
use std::thread;

#[macro_use]
extern crate serde_derive;

use anyhow::{Context as _, Error, Result};
use clap::{crate_version, Arg, ArgAction, Command};
use indexmap::IndexSet;
use itertools::Itertools;
use signal_hook::{consts::SIGINT, iterator::Signals};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod build;
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

fn determine_project_root(start: &Path) -> Result<(PathBuf, PathBuf)> {
    let mut cwd = start.to_owned();

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

fn clap() -> clap::Command {
    fn build_dir() -> Arg {
        Arg::new("build-dir")
            .help("specify build dir (relative to project root)")
            .short('B')
            .long("build-dir")
            .num_args(1)
            .value_name("DIR")
            .default_value("build")
            .value_parser(clap::value_parser!(PathBuf))
    }

    fn jobs() -> Arg {
        Arg::new("jobs")
            .help("how many compile jobs to run in parallel")
            .short('j')
            .long("jobs")
            .env("LAZE_JOBS")
            .num_args(1)
            .value_parser(clap::value_parser!(usize))
    }

    fn select() -> Arg {
        Arg::new("select")
            .help("extra modules to select/enable")
            .short('s')
            .long("select")
            .alias("enable")
            .env("LAZE_SELECT")
            .num_args(1..)
            .action(ArgAction::Append)
            .value_delimiter(',')
    }

    fn disable() -> Arg {
        Arg::new("disable")
            .help("disable modules")
            .short('d')
            .long("disable")
            .env("LAZE_DISABLE")
            .num_args(1..)
            .action(ArgAction::Append)
            .value_delimiter(',')
    }

    fn define() -> Arg {
        Arg::new("define")
            .help("set/override variable")
            .short('D')
            .long("define")
            .env("LAZE_DEFINE")
            .num_args(1..)
            .action(ArgAction::Append)
            .value_delimiter(',')
    }

    fn verbose() -> Arg {
        Arg::new("verbose")
            .help("be verbose (e.g., show command lines)")
            .short('v')
            .long("verbose")
            .action(ArgAction::Count)
    }

    fn partition() -> Arg {
        use std::str::FromStr;
        use task_partitioner::PartitionerBuilder;
        Arg::new("partition")
            .help("build only M/N subset (try \"count:1/2\")")
            .short('P')
            .long("partition")
            .num_args(1)
            .value_name("PARTITION")
            .value_parser(PartitionerBuilder::from_str)
    }

    Command::new("laze")
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
                .value_parser(clap::value_parser!(PathBuf))
                .num_args(1),
        )
        .arg(
            Arg::new("global")
                .short('g')
                .long("global")
                .help("global mode")
                .global(true)
                .env("LAZE_GLOBAL")
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("build")
                .about("generate build files and build")
                .arg(verbose())
                .arg(build_dir())
                .arg(
                    Arg::new("generate-only")
                        .short('G')
                        .long("generate-only")
                        .help("generate build files only, don't start build")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("compile-commands")
                        .short('c')
                        .long("compile-commands")
                        .env("LAZE_COMPILE_COMMANDS")
                        .help("generate compile_commands.json in project root")
                        .action(ArgAction::SetTrue),
                )
                .arg(jobs())
                .arg(
                    Arg::new("builders")
                        .short('b')
                        .long("builders")
                        .help("builders to configure")
                        .env("LAZE_BUILDERS")
                        .num_args(1..)
                        .action(ArgAction::Append)
                        .value_delimiter(','),
                )
                .arg(
                    Arg::new("apps")
                        .short('a')
                        .long("apps")
                        .help("apps to configure")
                        .env("LAZE_APPS")
                        .num_args(1..)
                        .action(ArgAction::Append)
                        .value_delimiter(','),
                )
                .arg(select())
                .arg(disable())
                .arg(define())
                .arg(partition()),
        )
        .subcommand(
            Command::new("task")
                .about("run builder specific task")
                .override_usage("laze task [FLAGS] [OPTIONS] <TASK> [ARGS]...")
                .allow_external_subcommands(true)
                .subcommand_required(true)
                .arg(build_dir())
                .arg(verbose())
                .arg(jobs())
                .arg(
                    Arg::new("builder")
                        .short('b')
                        .long("builder")
                        .help("builder to run task for")
                        .required(false)
                        .num_args(1)
                        .env("LAZE_BUILDERS"),
                )
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .help("application target to run task for")
                        .required(false)
                        .num_args(1)
                        .env("LAZE_APPS"),
                )
                .arg(select())
                .arg(disable())
                .arg(define()),
        )
        .subcommand(
            Command::new("clean")
                .about("clean current configuration")
                .arg(build_dir())
                .arg(verbose())
                .arg(
                    Arg::new("unused")
                        .short('u')
                        .long("unused")
                        .help(
                            "clean built files that are not produced by the current configuration",
                        )
                        .action(ArgAction::SetTrue),
                ),
        )
}

fn try_main() -> Result<i32> {
    let mut signals = Signals::new([SIGINT])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            if sig == SIGINT && !IGNORE_SIGINT.load(std::sync::atomic::Ordering::SeqCst) {
                std::process::exit(130);
            }
        }
    });

    let matches = clap().get_matches();

    if let Some(dir) = matches.get_one::<PathBuf>("chdir") {
        env::set_current_dir(dir)
            .context(format!("cannot change to directory \"{}\"", dir.display()))?;
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

    let global = matches.get_flag("global");
    env::set_current_dir(&project_root)
        .context(format!("cannot change to \"{}\"", &project_root.display()))?;

    match matches.subcommand() {
        Some(("build", build_matches)) => {
            let verbose = build_matches.get_count("verbose");
            let build_dir = build_matches.get_one::<PathBuf>("build-dir").unwrap();

            // collect builder names from args
            let builders = match build_matches.get_many::<String>("builders") {
                Some(values) => Selector::Some(values.cloned().collect::<IndexSet<String>>()),
                None => Selector::All,
            };

            // collect app names from args
            let apps = match build_matches.get_many::<String>("apps") {
                Some(values) => Selector::Some(values.cloned().collect::<IndexSet<String>>()),
                None => Selector::All,
            };

            let jobs = build_matches.get_one::<usize>("jobs").copied();

            let partitioner = build_matches
                .get_one::<task_partitioner::PartitionerBuilder>("partition")
                .map(|v| v.build());

            println!("laze: building {} for {}", &apps, &builders);

            // collect CLI selected/disabled modules
            let select = get_selects(build_matches);
            let disable = get_disables(build_matches);

            // collect CLI env overrides
            let cli_env = get_cli_vars(build_matches)?;

            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath),
            };

            let generator = GeneratorBuilder::default()
                .project_root(project_root.clone())
                .project_file(project_file)
                .build_dir(build_dir.clone())
                .mode(mode.clone())
                .builders(builders.clone())
                .apps(apps.clone())
                .select(select)
                .disable(disable)
                .cli_env(cli_env)
                .partitioner(partitioner.as_ref().map(|x| format!("{:?}", x)))
                .build()
                .unwrap();

            // arguments parsed, launch generation of ninja file(s)
            let builds = generator.execute(partitioner)?;

            let ninja_build_file = get_ninja_build_file(build_dir, &mode);

            if build_matches.get_flag("compile-commands") {
                let mut compile_commands = project_root;
                compile_commands.push("compile_commands.json");
                println!("generating {}", &compile_commands.to_string_lossy());
                ninja::generate_compile_commands(
                    ninja_build_file.as_path(),
                    compile_commands.as_path(),
                )?;
            }

            // generation of ninja build file complete.
            // exit here if requested.
            if build_matches.get_flag("generate-only") {
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

            ninja_run(ninja_build_file.as_path(), verbose > 0, targets, jobs)?;
        }
        Some(("task", task_matches)) => {
            let verbose = task_matches.get_count("verbose");
            let build_dir = task_matches.get_one::<PathBuf>("build-dir").unwrap();

            let builder = task_matches.get_one::<String>("builder");
            let app = task_matches.get_one::<String>("app");

            let jobs = task_matches.get_one("jobs").copied();

            // collect CLI selected/disabled modules
            let select = get_selects(task_matches);
            let disable = get_disables(task_matches);

            // collect CLI env overrides
            let cli_env = get_cli_vars(task_matches)?;

            let (task, args) = match task_matches.subcommand() {
                Some((name, matches)) => {
                    let args = matches
                        .get_many::<std::ffi::OsString>("")
                        .into_iter()
                        .flatten()
                        .map(|v| str::from_utf8(v.as_bytes()).expect("task arg is invalid UTF8"))
                        .collect::<Vec<_>>();
                    (name, Some(args))
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
                .partitioner(None)
                .build()
                .unwrap();

            let builds = generator.execute(None)?;

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
            let verbose = clean_matches.get_count("verbose");
            let unused = clean_matches.get_flag("unused");
            let build_dir = clean_matches.get_one::<PathBuf>("build-dir").unwrap();
            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath),
            };
            let ninja_build_file = get_ninja_build_file(build_dir, &mode);
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

fn get_cli_vars(
    build_matches: &clap::ArgMatches,
) -> Result<Option<im::HashMap<String, nested_env::EnvKey>>, Error> {
    let cli_env = if let Some(entries) = build_matches.get_many::<String>("define") {
        let mut env = Env::new();

        for assignment in entries {
            env = nested_env::assign_from_string(env, assignment)?;
        }

        Some(env)
    } else {
        None
    };
    Ok(cli_env)
}

fn get_disables(build_matches: &clap::ArgMatches) -> Option<Vec<String>> {
    let disable = build_matches
        .get_many::<String>("disable")
        .map(|vr| vr.cloned().collect_vec());
    disable
}

fn get_selects(build_matches: &clap::ArgMatches) -> Option<Vec<Dependency<String>>> {
    let select = build_matches.get_many::<String>("select");
    // convert CLI --select strings to Vec<Dependency>
    select.map(|vr| vr.map(crate::data::dependency_from_string).collect_vec())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_clap() {
        crate::clap().debug_assert();
    }
}
