use std::path::PathBuf;

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
            .value_parser(clap::value_parser!(PathBuf))
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
                .value_hint(ValueHint::DirPath)
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
        .subcommand(
            Command::new("completion")
                .arg(
                    Arg::new("generator")
                        .long("generate")
                        .value_parser(value_parser!(clap_complete::Shell)),
                )
                .hide(true),
        )
}
