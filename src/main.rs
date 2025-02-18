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
use std::str;
use std::sync::{Arc, OnceLock};

#[macro_use]
extern crate serde_derive;

use anyhow::{Context as _, Error, Result};
use camino::{Utf8Path, Utf8PathBuf};
use git_cache::GitCache;
use indexmap::IndexSet;
use itertools::Itertools;
use signal_hook::{consts::SIGINT, flag::register_conditional_shutdown};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod build;
mod cli;
mod data;
mod download;
mod generate;
mod insights;
mod model;
mod nested_env;
mod new;
mod ninja;
mod serde_bool_helpers;
mod task_runner;
mod utils;

use model::{Context, ContextBag, Dependency, Module, Rule, Task, TaskError};

use generate::{get_ninja_build_file, BuildInfo, GenerateMode, GeneratorBuilder, Selector};
use nested_env::{Env, MergeOption};
use ninja::NinjaCmdBuilder;

pub static GIT_CACHE: OnceLock<GitCache> = OnceLock::new();

pub(crate) fn determine_project_root(start: &Utf8Path) -> Result<(Utf8PathBuf, Utf8PathBuf)> {
    let mut cwd = start.to_owned();

    loop {
        let mut tmp = cwd.clone();
        tmp.push("laze-project.yml");
        if tmp.exists() {
            return Ok((cwd, Utf8PathBuf::from("laze-project.yml")));
        }
        cwd = match cwd.parent() {
            Some(p) => Utf8PathBuf::from(p),
            None => return Err(anyhow!("cannot find laze-project.yml")),
        }
    }
}

fn ninja_run(
    ninja_buildfile: &Utf8Path,
    verbose: bool,
    targets: Option<Vec<Utf8PathBuf>>,
    jobs: Option<usize>,
    keep_going: Option<usize>,
) -> Result<i32, Error> {
    let mut ninja_cmd = NinjaCmdBuilder::default();

    ninja_cmd
        .verbose(verbose)
        .build_file(ninja_buildfile)
        .targets(targets);

    if let Some(jobs) = jobs {
        ninja_cmd.jobs(jobs);
    }

    if let Some(keep_going) = keep_going {
        ninja_cmd.keep_going(keep_going);
    }

    let ninja_cmd = ninja_cmd.build().unwrap();
    let ninja_binary = ninja_cmd.binary;
    let ninja_exit = ninja_cmd
        .run()
        .with_context(|| format!("launching ninja binary \"{}\"", ninja_binary))?;

    match ninja_exit.code() {
        Some(code) => match code {
            0 => Ok(code),
            _ => Err(anyhow!("ninja exited with code {code}")),
        },
        None => Err(anyhow!("ninja probably killed by signal")),
    }
}

fn main() {
    let result = try_main();
    match result {
        Err(e) => {
            if let Some(expr_err) = e.downcast_ref::<evalexpr::EvalexprError>() {
                // make expression errors more readable.
                // TODO: factor out
                eprintln!("laze: expression error: {expr_err}");
                eprintln!("laze: the error occured here:");
                let mut iter = e.chain().peekable();
                let mut i = 0;
                while let Some(next) = iter.next() {
                    if iter.peek().is_none() {
                        break;
                    }
                    eprintln!("{i:>5}: {next}");
                    i += 1;
                }
            } else {
                eprintln!("laze: error: {e:#}");
            }
            std::process::exit(1);
        }
        Ok(code) => std::process::exit(code),
    };
}

pub static EXIT_ON_SIGINT: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn try_main() -> Result<i32> {
    EXIT_ON_SIGINT.set(Arc::new(AtomicBool::new(true))).unwrap();
    register_conditional_shutdown(SIGINT, 130, EXIT_ON_SIGINT.get().unwrap().clone()).unwrap();

    clap_complete::env::CompleteEnv::with_factory(cli::clap).complete();

    let matches = cli::clap().get_matches();

    let git_cache_dir = Utf8PathBuf::from(&shellexpand::tilde(
        matches.get_one::<Utf8PathBuf>("git_cache_dir").unwrap(),
    ));

    GIT_CACHE
        .set(GitCache::new(git_cache_dir)?)
        .ok()
        .expect("creating git cache directory.");

    // handle project independent subcommands here
    match matches.subcommand() {
        Some(("new", matches)) => {
            new::from_matches(matches)?;
            return Ok(0);
        }
        Some(("completion", matches)) => {
            fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
                clap_complete::generate(
                    gen,
                    cmd,
                    cmd.get_name().to_string(),
                    &mut std::io::stdout(),
                );
            }
            if let Some(generator) = matches
                .get_one::<clap_complete::Shell>("generator")
                .copied()
            {
                let mut cmd = cli::clap();
                eprintln!("Generating completion file for {}...", generator);
                print_completions(generator, &mut cmd);
            }
            return Ok(0);
        }
        Some(("manpages", matches)) => {
            fn create_manpage(cmd: clap::Command, outfile: &Utf8Path) -> Result<(), Error> {
                let man = clap_mangen::Man::new(cmd);
                let mut buffer: Vec<u8> = Default::default();
                man.render(&mut buffer)?;

                std::fs::write(outfile, buffer)?;
                Ok(())
            }
            let mut outpath: Utf8PathBuf =
                matches.get_one::<Utf8PathBuf>("outdir").unwrap().clone();
            let cmd = cli::clap();

            outpath.push("laze.1");
            create_manpage(cmd.clone(), &outpath)?;

            for subcommand in cmd.get_subcommands() {
                if subcommand.is_hide_set() {
                    continue;
                }
                let name = subcommand.get_name();
                outpath.pop();
                outpath.push(format!("laze-{name}.1"));
                create_manpage(subcommand.clone(), &outpath)?;
            }

            return Ok(0);
        }
        Some(("git-clone", matches)) => {
            let repository = matches.get_one::<String>("repository").unwrap();
            let target_path = matches.get_one::<Utf8PathBuf>("target_path").cloned();
            let wanted_commit = matches.get_one::<String>("commit");
            let sparse_paths = matches
                .get_many::<String>("sparse-add")
                .map(|v| v.into_iter().cloned().collect::<Vec<String>>());

            GIT_CACHE
                .get()
                .unwrap()
                .cloner()
                .commit(wanted_commit.cloned())
                .extra_clone_args_from_matches(matches)
                .repository_url(repository.clone())
                .sparse_paths(sparse_paths)
                .target_path(target_path)
                .update(matches.get_flag("update"))
                .do_clone()?;

            return Ok(0);
        }
        _ => (),
    }

    if let Some(dir) = matches.get_one::<Utf8PathBuf>("chdir") {
        env::set_current_dir(dir).context(format!("cannot change to directory \"{dir}\""))?;
    }

    let cwd = Utf8PathBuf::try_from(env::current_dir()?).expect("cwd not UTF8");

    let (project_root, project_file) = determine_project_root(&cwd)?;
    let start_relpath = pathdiff::diff_utf8_paths(&cwd, &project_root).unwrap();
    let start_relpath = if start_relpath.eq("") {
        ".".into()
    } else {
        start_relpath
    };

    println!(
        "laze: project root: {project_root} relpath: {start_relpath} project_file: {project_file}",
    );

    let global = matches.get_flag("global");
    env::set_current_dir(&project_root).context(format!("cannot change to \"{project_root}\""))?;

    let verbose = matches.get_count("verbose");

    match matches.subcommand() {
        Some(("build", build_matches)) => {
            let build_dir = build_matches.get_one::<Utf8PathBuf>("build-dir").unwrap();

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
            let keep_going = build_matches.get_one::<usize>("keep_going").copied();

            let partitioner = build_matches
                .get_one::<task_partitioner::PartitionerBuilder>("partition")
                .map(|v| v.build());

            let info_outfile = build_matches.get_one::<Utf8PathBuf>("info-export");

            println!("laze: building {apps} for {builders}");

            // collect CLI selected/disabled modules
            let select = get_selects(build_matches);
            let disable = get_disables(build_matches);

            // collect CLI env overrides
            let cli_env = get_cli_vars(build_matches)?;

            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath.clone()),
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
                .collect_insights(info_outfile.is_some())
                .disable_cache(info_outfile.is_some())
                .build()
                .unwrap();

            // arguments parsed, launch generation of ninja file(s)
            let builds = generator.execute(partitioner)?;

            if let Some(info_outfile) = info_outfile {
                use std::fs::File;
                use std::io::BufWriter;
                let info_outfile = start_relpath.join(info_outfile);
                let insights = insights::Insights::from_builds(&builds.build_infos);
                let buffer =
                    BufWriter::new(File::create(&info_outfile).with_context(|| {
                        format!("creating info export file \"{info_outfile}\"")
                    })?);
                serde_json::to_writer_pretty(buffer, &insights)
                    .with_context(|| "exporting build info".to_string())?;
            }

            let ninja_build_file = get_ninja_build_file(build_dir, &mode);

            if build_matches.get_flag("compile-commands") {
                let mut compile_commands = project_root.clone();
                compile_commands.push("compile_commands.json");
                println!("laze: generating {compile_commands}");
                ninja::generate_compile_commands(&ninja_build_file, &compile_commands)?;
            }

            // collect (optional) task and it's arguments
            let task = collect_tasks(build_matches);

            // generation of ninja build file complete.
            // exit here if requested.
            if task.is_none() && build_matches.get_flag("generate-only") {
                return Ok(0);
            }

            if let Some((task, args)) = task {
                let builds: Vec<&BuildInfo> = builds
                    .build_infos
                    .iter()
                    .filter(|build_info| {
                        builders.selects(&build_info.builder)
                            && apps.selects(&build_info.binary)
                            && build_info.tasks.contains_key(task)
                    })
                    .collect();

                if !builds
                    .iter()
                    .any(|build_info| build_info.tasks.iter().any(|t| t.1.is_ok() && t.0 == task))
                {
                    let mut not_available = 0;
                    for b in builds {
                        for t in &b.tasks {
                            if t.1.is_err() && t.0 == task {
                                not_available += 1;
                                if verbose > 0 {
                                    eprintln!(
                                    "laze: warn: task \"{task}\" for binary \"{}\" on builder \"{}\": {}",
                                    b.binary,
                                    b.builder,
                                    t.1.as_ref().err().unwrap()
                                );
                                }
                            }
                        }
                    }

                    if not_available > 0 && verbose == 0 {
                        println!("laze hint: {not_available} target(s) not available, try `--verbose` to list why");
                    }
                    return Err(anyhow!("no matching target for task \"{}\" found.", task));
                }

                let multiple = build_matches.get_flag("multiple");

                if builds.len() > 1 && !multiple {
                    println!("laze: multiple task targets found:");
                    for build_info in builds {
                        eprintln!("{} {}", build_info.builder, build_info.binary);
                    }

                    // TODO: allow running tasks for multiple targets
                    return Err(anyhow!(
                        "please specify one of these builders, or -m/--multiple-tasks."
                    ));
                }

                let task_name = task;
                let mut targets = Vec::new();
                let mut ninja_targets = Vec::new();

                for build in builds {
                    let task = build.tasks.get(task).unwrap();
                    if let Ok(task) = task {
                        if task.build_app() {
                            let build_target = build.out.clone();
                            ninja_targets.push(build_target);
                        }
                        targets.push((build, task));
                    }
                }

                if !ninja_targets.is_empty() && !build_matches.get_flag("generate-only") {
                    let ninja_build_file = get_ninja_build_file(build_dir, &mode);
                    if ninja_run(
                        ninja_build_file.as_path(),
                        verbose > 0,
                        Some(ninja_targets),
                        jobs,
                        None, // have to fail on build error b/c no way of knowing *which* target
                              // failed
                    )? != 0
                    {
                        return Err(anyhow!("build error"));
                    };
                }

                let (results, errors) = task_runner::run_tasks(
                    task_name,
                    targets.iter(),
                    args.as_ref(),
                    verbose,
                    keep_going.unwrap(),
                    project_root.as_std_path(),
                )?;

                if errors > 0 {
                    if multiple {
                        // multiple tasks, more than zero errors. print them
                        println!("laze: the following tasks failed:");
                        for result in results.iter().filter(|r| r.result.is_err()) {
                            println!(
                                "laze: task \"{task_name}\" on app \"{}\" for builder \"{}\"",
                                result.build.binary, result.build.builder
                            );
                        }
                    }
                    return Ok(1);
                }
            } else {
                // build ninja target arguments, if necessary
                let targets: Option<Vec<Utf8PathBuf>> = if let Selector::All = builders {
                    if let Selector::All = apps {
                        None
                    } else {
                        // TODO: filter by app
                        None
                    }
                } else {
                    Some(
                        builds
                            .build_infos
                            .iter()
                            .filter_map(|build_info| {
                                (builders.selects(&build_info.builder)
                                    && apps.selects(&build_info.binary))
                                .then_some(build_info.out.clone())
                            })
                            .collect(),
                    )
                };

                ninja_run(
                    ninja_build_file.as_path(),
                    verbose > 0,
                    targets,
                    jobs,
                    keep_going,
                )?;
            }
        }
        Some(("clean", clean_matches)) => {
            let unused = clean_matches.get_flag("unused");
            let build_dir = clean_matches.get_one::<Utf8PathBuf>("build-dir").unwrap();
            let mode = match global {
                true => GenerateMode::Global,
                false => GenerateMode::Local(start_relpath),
            };
            let ninja_build_file = get_ninja_build_file(build_dir, &mode);
            let tool = match unused {
                true => "cleandead",
                false => "clean",
            };
            let clean_target: Option<Vec<Utf8PathBuf>> = Some(vec!["-t".into(), tool.into()]);
            ninja_run(
                ninja_build_file.as_path(),
                verbose > 0,
                clean_target,
                None,
                None,
            )?;
        }
        _ => {}
    };

    Ok(0)
}

fn collect_tasks(task_matches: &clap::ArgMatches) -> Option<(&str, Option<Vec<&str>>)> {
    match task_matches.subcommand() {
        Some((name, matches)) => {
            let args = matches
                .get_many::<std::ffi::OsString>("")
                .into_iter()
                .flatten()
                .map(|v| v.as_os_str().to_str().expect("task arg is invalid UTF8"))
                .collect::<Vec<_>>();
            Some((name, Some(args)))
        }
        _ => None,
    }
}

fn get_cli_vars(build_matches: &clap::ArgMatches) -> Result<Option<Env>, Error> {
    let cli_env = if let Some(entries) = build_matches.get_many::<String>("define") {
        let mut env = Env::new();

        for assignment in entries {
            env.assign_from_string(assignment)?;
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
        crate::cli::clap().debug_assert();
    }
}
