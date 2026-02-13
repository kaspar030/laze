use core::sync::atomic::AtomicBool;
use std::env;
use std::io::Write;
use std::str;
use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, Context as _, Error, Result};
use camino::{Utf8Path, Utf8PathBuf};
use git_cache::GitCache;
use itertools::Itertools;
use jobserver::JOBSERVER;
use log::{debug, error, info, log_enabled, Level::Debug, LevelFilter};
use signal_hook::{consts::SIGINT, flag::register_conditional_shutdown};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod build;
mod cli;
mod data;
mod download;
mod generate;
mod insights;
mod inspect;
mod jobserver;
mod model;
mod nested_env;
mod new;
mod ninja;
mod serde_bool_helpers;
mod subst_ext;
mod task_runner;
mod utils;

use inspect::BuildInspector;
use model::{Context, ContextBag, ContextBagError, Dependency, Module, Rule, Task, TaskError};

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
    targets: Option<Vec<Utf8PathBuf>>,
    jobs: Option<usize>,
    keep_going: Option<usize>,
) -> Result<i32, Error> {
    let mut ninja_cmd = NinjaCmdBuilder::default();

    ninja_cmd
        .build_file(ninja_buildfile)
        .verbose(log_enabled!(Debug))
        .targets(targets);

    if let Some(jobs) = jobs {
        ninja_cmd.jobs(jobs);
    }

    if let Some(keep_going) = keep_going {
        ninja_cmd.keep_going(keep_going);
    }

    let ninja_cmd = ninja_cmd.build().unwrap();
    let ninja_binary = ninja_cmd.binary;

    let ninja_exit = if jobs.is_some() {
        // we force some `-jN`
        ninja_cmd.cmd().status()
    } else if let Some(jobserver) = JOBSERVER.get() {
        // we use our own jobserver
        jobserver.configure_make_and_run_with_fifo(&mut ninja_cmd.cmd(), |cmd| cmd.status())
    } else {
        // our jobserver is not available (e.g., on `laze clean`)
        ninja_cmd.cmd().status()
    }
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
                error!("laze: expression error: {expr_err}");
                error!("laze: the error occured here:");
                let mut iter = e.chain().peekable();
                let mut i = 0;
                while let Some(next) = iter.next() {
                    if iter.peek().is_none() {
                        break;
                    }
                    error!("{i:>5}: {next}");
                    i += 1;
                }
            } else {
                error!("laze: error: {e:#}");
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

    // Set up the logger
    let env = env_logger::Env::default().filter("LAZE_LOG_LEVEL");
    let mut env_log_builder = env_logger::Builder::from_env(env);
    let log_builder = env_log_builder.format(|buf, record| writeln!(buf, "{}", record.args()));

    let quiet = matches.get_count("quiet");
    let verbose = matches.get_count("verbose");
    match (verbose, quiet) {
        (1, ..) => log_builder.filter_level(LevelFilter::Debug),
        (2.., ..) => log_builder.filter_level(LevelFilter::max()),
        (0, 1) => log_builder.filter_level(LevelFilter::Warn),
        (0, 2..) => log_builder.filter_level(LevelFilter::Error),
        (0, 0) => log_builder.filter_level(LevelFilter::Info),
    }
    .init();

    let git_cache_dir = Utf8PathBuf::from(&shellexpand::tilde(
        matches.get_one::<Utf8PathBuf>("git_cache_dir").unwrap(),
    ));

    GIT_CACHE
        .set(GitCache::new(git_cache_dir)?)
        .ok()
        .expect("creating git cache directory.");

    // handle project independent subcommands here
    match matches.subcommand() {
        Some(("new", matches)) => cmd_new(matches),
        Some(("completion", matches)) => cmd_completion(matches),
        Some(("manpages", matches)) => cmd_manpages(matches),
        Some(("git-clone", matches)) => cmd_gitclone(matches),
        _ => try_main_build(matches),
    }
}

fn try_main_build(matches: clap::ArgMatches) -> Result<i32> {
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

    info!(
        "laze: project root: {project_root} relpath: {start_relpath} project_file: {project_file}",
    );

    let global = matches.get_flag("global");
    env::set_current_dir(&project_root).context(format!("cannot change to \"{project_root}\""))?;

    // If there's a parent jobserver, get it now. Needs to be done early.
    jobserver::maybe_init_fromenv();

    match matches.subcommand() {
        Some(("build", matches)) => {
            cmd_build(matches, global, project_root, project_file, start_relpath)
        }
        Some(("inspect", matches)) => cmd_inspect(matches, project_file),
        Some(("clean", matches)) => cmd_clean(matches, global, start_relpath),
        _ => Ok(0),
    }
}

fn cmd_new(matches: &clap::ArgMatches) -> Result<i32> {
    new::from_matches(matches)?;
    Ok(0)
}

fn cmd_completion(matches: &clap::ArgMatches) -> Result<i32> {
    fn print_completions<G: clap_complete::Generator>(generator: G, cmd: &mut clap::Command) {
        clap_complete::generate(
            generator,
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
        print_completions(generator, &mut cmd);
    }
    Ok(0)
}

fn cmd_manpages(matches: &clap::ArgMatches) -> Result<i32> {
    fn create_manpage(cmd: clap::Command, outfile: &Utf8Path) -> Result<(), Error> {
        let man = clap_mangen::Man::new(cmd);
        let mut buffer: Vec<u8> = Default::default();
        man.render(&mut buffer)?;

        std::fs::write(outfile, buffer)?;
        Ok(())
    }
    let mut outpath: Utf8PathBuf = matches.get_one::<Utf8PathBuf>("outdir").unwrap().clone();
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

fn cmd_gitclone(matches: &clap::ArgMatches) -> Result<i32> {
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

fn cmd_build(
    matches: &clap::ArgMatches,
    global: bool,
    project_root: Utf8PathBuf,
    project_file: Utf8PathBuf,
    start_relpath: Utf8PathBuf,
) -> Result<i32> {
    let build_dir = matches.get_one::<Utf8PathBuf>("build-dir").unwrap();

    // collect builder names from args
    let builders = Selector::from(matches.get_many::<String>("builders"));
    // collect app names from args
    let apps = Selector::from(matches.get_many::<String>("apps"));

    let jobs = matches.get_one::<usize>("jobs").copied();

    // Unless we've inherited a jobserver, create one.
    jobserver::maybe_set_limit(jobs.unwrap_or_else(|| {
        // default to number of logical cores.
        // TODO: figure out in which case this might error
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }));

    let keep_going = matches.get_one::<usize>("keep_going").copied();

    let partitioner = matches
        .get_one::<task_partitioner::PartitionerBuilder>("partition")
        .map(|v| v.build());

    let info_outfile = matches.get_one::<Utf8PathBuf>("info-export");

    debug!("laze: building {apps} for {builders}");

    // collect CLI selected/disabled modules
    let select = get_selects(matches);
    let disable = get_disables(matches);
    let require = get_requires(matches);

    // collect CLI env overrides
    let cli_env = get_cli_vars(matches)?;

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
        .require(require)
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
        let buffer = BufWriter::new(
            File::create(&info_outfile)
                .with_context(|| format!("creating info export file \"{info_outfile}\""))?,
        );
        serde_json::to_writer_pretty(buffer, &insights)
            .with_context(|| "exporting build info".to_string())?;
    }

    let ninja_build_file = get_ninja_build_file(build_dir, &mode);

    if matches.get_flag("compile-commands") {
        let mut compile_commands = project_root.clone();
        compile_commands.push("compile_commands.json");
        ninja::generate_compile_commands(&ninja_build_file, &compile_commands)?;
        info!("laze: generated {compile_commands}");
    }

    // collect (optional) task and it's arguments
    let task = collect_tasks(matches);

    // generation of ninja build file complete.
    // exit here if requested.
    if task.is_none() && matches.get_flag("generate-only") {
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
                        debug!(
                            "laze: warn: task \"{task}\" for binary \"{}\" on builder \"{}\": {}",
                            b.binary,
                            b.builder,
                            t.1.as_ref().err().unwrap()
                        );
                    }
                }
            }

            if not_available > 0 {
                info!("laze hint: {not_available} target(s) not available, try `--verbose` to list why");
            }
            return Err(anyhow!("no matching target for task \"{}\" found.", task));
        }

        let multiple = matches.get_flag("multiple");

        if builds.len() > 1 && !multiple {
            info!("laze: multiple task targets found:");
            for build_info in builds {
                info!("{} {}", build_info.builder, build_info.binary);
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

        if !ninja_targets.is_empty() && !matches.get_flag("generate-only") {
            let ninja_build_file = get_ninja_build_file(build_dir, &mode);
            if ninja_run(
                ninja_build_file.as_path(),
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
            keep_going.unwrap(),
            project_root.as_std_path(),
        )?;

        if errors > 0 {
            if multiple {
                // multiple tasks, more than zero errors. print them
                error!("laze: the following tasks failed:");
                for result in results.iter().filter(|r| r.result.is_err()) {
                    error!(
                        "laze: task \"{task_name}\" on app \"{}\" for builder \"{}\"",
                        result.build.binary, result.build.builder
                    );
                }
            } else {
                // only one error. can't move out of first, cant clone, so print that here.
                let (first, _rest) = results.split_first().unwrap();
                if let Err(e) = &first.result {
                    error!("laze: error: {e:#}");
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
                        (builders.selects(&build_info.builder) && apps.selects(&build_info.binary))
                            .then_some(build_info.out.clone())
                    })
                    .collect(),
            )
        };

        ninja_run(ninja_build_file.as_path(), targets, jobs, keep_going)?;
    }
    Ok(0)
}

fn cmd_clean(matches: &clap::ArgMatches, global: bool, start_relpath: Utf8PathBuf) -> Result<i32> {
    let unused = matches.get_flag("unused");
    let build_dir = matches.get_one::<Utf8PathBuf>("build-dir").unwrap();
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
    ninja_run(ninja_build_file.as_path(), clean_target, None, None)?;
    Ok(0)
}

fn cmd_inspect(matches: &clap::ArgMatches, project_file: Utf8PathBuf) -> Result<i32> {
    let build_dir = matches.get_one::<Utf8PathBuf>("build-dir").unwrap();
    match matches.subcommand() {
        Some(("builders", matches)) => {
            let build_inspector = BuildInspector::from_project(project_file, build_dir.clone())?;
            if matches.get_flag("tree") {
                build_inspector.write_tree(&std::io::stdout())?;
            } else {
                let builders = build_inspector.inspect_builders();
                builders
                    .iter()
                    .for_each(|builder| println!("{}", builder.name));
            }
        }
        _ => (),
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
    get_string_arg_vec(build_matches, "disable")
}

fn get_requires(build_matches: &clap::ArgMatches) -> Option<Vec<String>> {
    get_string_arg_vec(build_matches, "require")
}

fn get_selects(build_matches: &clap::ArgMatches) -> Option<Vec<Dependency<String>>> {
    let select = build_matches.get_many::<String>("select");
    // convert CLI --select strings to Vec<Dependency>
    select.map(|vr| vr.map(crate::data::dependency_from_string).collect_vec())
}

fn get_string_arg_vec(build_matches: &clap::ArgMatches, id: &str) -> Option<Vec<String>> {
    let res = build_matches
        .get_many::<String>(id)
        .map(|vr| vr.cloned().collect_vec());
    res
}

#[cfg(test)]
mod test {
    #[test]
    fn test_clap() {
        crate::cli::clap().debug_assert();
    }
}
