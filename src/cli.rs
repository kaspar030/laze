use camino::Utf8PathBuf;

use clap::{crate_version, value_parser, Arg, ArgAction, Command, ValueHint};

pub fn clap() -> clap::Command {
    fn build_dir() -> Arg {
        Arg::new("build-dir")
            .help("specify build dir (relative to project root)")
            .short('B')
            .long("build-dir")
            .num_args(1)
            .value_name("DIR")
            .default_value("build")
            .value_parser(clap::value_parser!(Utf8PathBuf))
            .value_hint(ValueHint::DirPath)
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
            .action(ArgAction::Append)
            .value_delimiter(',')
    }

    fn disable() -> Arg {
        Arg::new("disable")
            .help("disable modules")
            .short('d')
            .long("disable")
            .env("LAZE_DISABLE")
            .action(ArgAction::Append)
            .value_delimiter(',')
    }

    fn define() -> Arg {
        Arg::new("define")
            .help("set/override variable")
            .short('D')
            .long("define")
            .env("LAZE_DEFINE")
            .action(ArgAction::Append)
            .value_delimiter(',')
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
                .value_parser(clap::value_parser!(Utf8PathBuf))
                .value_hint(ValueHint::DirPath)
                .num_args(1),
        )
        .arg(
            Arg::new("verbose")
                .help("be verbose (e.g., show command lines)")
                .short('v')
                .long("verbose")
                .global(true)
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("quiet")
                .help("do not print laze log messages")
                .short('q')
                .long("quiet")
                .global(true)
                .action(ArgAction::Count)
                .hide(true), // (not really supported, yet)
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
        .arg(git_cache::clap_git_cache_dir_arg())
        .subcommand(
            Command::new("build")
                .about("generate build files and build")
                .allow_external_subcommands(true)
                .override_usage("laze build [OPTIONS] [<TASK> [ARGS]...]")
                .next_help_heading("Build options")
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
                .arg(
                    Arg::new("info-export")
                        .short('i')
                        .long("info-export")
                        .env("LAZE_DUMP_INFO")
                        .help("export build info to file (as JSON)")
                        .value_parser(clap::value_parser!(Utf8PathBuf))
                        .value_hint(ValueHint::FilePath)
                        .num_args(1),
                )
                .arg(
                    Arg::new("multiple")
                        .short('m')
                        .long("multiple-tasks")
                        .env("LAZE_MULTIPLE_TASKS")
                        .help("if multiple tasks targets are available, execute them all")
                        .action(ArgAction::SetTrue),
                )
                .arg(jobs())
                .next_help_heading("What to build")
                .arg(
                    Arg::new("builders")
                        .short('b')
                        .long("builders")
                        .help("builders to configure")
                        .env("LAZE_BUILDERS")
                        .action(ArgAction::Append)
                        .value_delimiter(','),
                )
                .arg(
                    Arg::new("apps")
                        .short('a')
                        .long("apps")
                        .help("apps to configure")
                        .env("LAZE_APPS")
                        .action(ArgAction::Append)
                        .value_delimiter(','),
                )
                .arg(partition())
                .next_help_heading("Extra build settings")
                .arg(select())
                .arg(disable())
                .arg(define()),
        )
        .subcommand(
            Command::new("clean")
                .about("clean current configuration")
                .arg(build_dir())
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
        .subcommand(
            Command::new("new")
                .about("Create a new laze project at <PATH>")
                .arg(
                    Arg::new("path")
                        .help("folder for new project")
                        .value_parser(value_parser!(Utf8PathBuf))
                        .value_hint(ValueHint::DirPath)
                        .required(true),
                )
                .arg(
                    Arg::new("template")
                        .short('t')
                        .long("template")
                        .help("template to use for new project")
                        .required(false)
                        .default_value("default")
                        .value_parser(clap::value_parser!(String))
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("completion")
                .about("Generate laze shell completions.")
                .arg(
                    Arg::new("generator")
                        .help("shell to generate completions for")
                        .long("generate")
                        .value_parser(value_parser!(clap_complete::Shell)),
                )
                .hide(true),
        )
        .subcommand(
            Command::new("manpages")
                .about("Generate laze manpages.")
                .arg(
                    Arg::new("outdir")
                        .help("directory in which to create manpage files")
                        .value_parser(value_parser!(Utf8PathBuf))
                        .value_hint(ValueHint::DirPath)
                        .required(true),
                )
                .hide(true),
        )
        .subcommand(git_cache::clap_clone_command("git-clone").hide(true))
}
