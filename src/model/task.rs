use std::cell::LazyCell;
use std::ffi::OsStr;
use std::path::Path;

use anyhow::{Context, Error, Result};
use im::Vector;
use indexmap::IndexMap;
use itertools::Itertools;
use log::debug;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::nested_env::{self, EnvMap};
use crate::serde_bool_helpers::{default_as_false, default_as_true};
use crate::subst_ext::{substitute, IgnoreMissing, LocalVec};
use crate::EXIT_ON_SIGINT;

use super::shared::VarExportSpec;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub cmd: Vec<String>,
    pub help: Option<String>,
    pub required_vars: Option<Vec<String>>,
    pub required_modules: Option<Vec<String>>,
    pub export: Option<Vector<VarExportSpec>>,
    #[serde(default = "default_as_true")]
    pub build: bool,
    #[serde(default = "default_as_false")]
    pub ignore_ctrl_c: bool,
    pub workdir: Option<String>,
}

#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum TaskError {
    #[error("required variable `{var}` not set")]
    RequiredVarMissing { var: String },
    #[error("required module `{module}` not selected")]
    RequiredModuleMissing { module: String },
    #[error("task command had non-zero exit code")]
    CmdExitError,
    #[error("task `{task_name}` not found")]
    TaskNotFound { task_name: String },
}

impl Task {
    pub fn build_app(&self) -> bool {
        self.build
    }

    pub fn execute(
        &self,
        start_dir: &Path,
        args: Option<&Vec<&str>>,
        all_tasks: &IndexMap<String, Result<Task, TaskError>>,
        parent_exports: &Vector<VarExportSpec>,
    ) -> Result<(), Error> {
        let argv_str = LazyCell::new(|| match args {
            None => "".into(),
            Some(v) if v.is_empty() => "".into(),
            Some(v) => format!(" argv: {:?}", v),
        });

        for cmd_full in &self.cmd {
            debug!("laze: command: `{cmd_full}`{}", *argv_str);

            if let Some(cmd) = cmd_full.strip_prefix(":") {
                let cmd = create_cmd_vec(cmd, args);

                self.execute_subtask(cmd, start_dir, all_tasks, parent_exports)
            } else {
                self.execute_shell_cmd(cmd_full, args, start_dir, parent_exports)
            }
            .with_context(|| format!("command `{cmd_full}`"))?;
        }
        Ok(())
    }

    fn execute_shell_cmd(
        &self,
        cmd: &str,
        args: Option<&Vec<&str>>,
        start_dir: &Path,
        parent_exports: &Vector<VarExportSpec>,
    ) -> Result<(), Error> {
        use std::process::Command;
        let mut command = if cfg!(target_family = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C");
            cmd
        } else {
            let mut sh = Command::new("sh");
            sh.arg("-c");
            sh
        };

        if let Some(working_directory) = &self.workdir {
            // This includes support for absolute working directories through .join
            command.current_dir(start_dir.join(working_directory));
        } else {
            command.current_dir(start_dir);
        }

        // handle "export:" (export laze variables to task shell environment)
        for entry in parent_exports
            .into_iter()
            .chain(self.export.iter().flatten())
        {
            let VarExportSpec { variable, content } = entry;
            if let Some(val) = content {
                command.env(variable, val);
            }
        }

        // TODO: is this still needed?
        let cmd = cmd.replace("$$", "$");

        command.arg(cmd);

        if let Some(args) = args {
            if !args.is_empty() {
                command.arg("--");
                command.args(args);
            }
        }

        let mut full_command = Vec::new();
        full_command.push(command.get_program().to_string_lossy());
        full_command.extend(command.get_args().map(OsStr::to_string_lossy));

        debug!("laze: executing `{full_command:?}`");

        if self.ignore_ctrl_c {
            EXIT_ON_SIGINT
                .get()
                .unwrap()
                .clone()
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
        // run command, wait for status
        let status = command.status().expect("executing command");

        if self.ignore_ctrl_c {
            EXIT_ON_SIGINT
                .get()
                .unwrap()
                .clone()
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        status
            .success()
            .then_some(())
            .ok_or(TaskError::CmdExitError.into())
    }

    fn execute_subtask(
        &self,
        cmd: Vec<String>,
        start_dir: &Path,
        all_tasks: &IndexMap<String, std::result::Result<Task, TaskError>>,
        parent_exports: &Vector<VarExportSpec>,
    ) -> Result<(), Error> {
        // turn cmd into proper `Vec<&str>` without the command name.
        let args = cmd.iter().skip(1).map(|s| s.as_str()).collect_vec();

        let task_name = &cmd[0];

        // resolve task name to task
        let other_task = all_tasks
            .get(task_name)
            .ok_or_else(|| TaskError::TaskNotFound {
                task_name: task_name.clone(),
            })?
            .as_ref()
            .map_err(|e| e.clone())
            .with_context(|| format!("task '{task_name}'"))?;

        let mut parent_exports = parent_exports.clone();
        if let Some(export) = self.export.as_ref() {
            parent_exports.append(export.clone());
        }

        other_task.execute(start_dir, Some(&args), all_tasks, &parent_exports)?;

        Ok(())
    }

    fn _with_env(&self, env: &EnvMap, do_eval: bool) -> Result<Task, Error> {
        let expand = |s| {
            if do_eval {
                nested_env::expand_eval(s, env, nested_env::IfMissing::Empty)
            } else {
                nested_env::expand(s, env, nested_env::IfMissing::Ignore)
            }
        };

        Ok(Task {
            cmd: self
                .cmd
                .iter()
                .map(expand)
                .collect::<Result<Vec<String>, _>>()?,
            export: if do_eval {
                self.expand_export(env)
            } else {
                self.export.clone()
            },
            workdir: self.workdir.as_ref().map(expand).transpose()?,
            ..(*self).clone()
        })
    }

    /// This is called early when loading the yaml files.
    /// It will not evaluate expressions, and pass-through variables that are not
    /// found in `env`.
    pub fn with_env(&self, env: &EnvMap) -> Result<Task, Error> {
        self._with_env(env, false)
    }

    /// This is called to generate the final task.
    /// It will evaluate expressions, and variables that are not
    /// found in `env` will be replaced with the empty string.
    pub fn with_env_eval(&self, env: &EnvMap) -> Result<Task, Error> {
        self._with_env(env, true)
    }

    fn expand_export(&self, env: &EnvMap) -> Option<Vector<VarExportSpec>> {
        self.export
            .as_ref()
            .map(|export| VarExportSpec::expand(export.iter(), env))
    }
}

fn create_cmd_vec(cmd: &str, args: Option<&Vec<&str>>) -> Vec<String> {
    let mut cmd = shell_words::split(cmd).unwrap();
    if let Some(args) = args {
        substitute_args(&mut cmd, args);
    }
    cmd
}

fn substitute_args(cmd: &mut Vec<String>, args: &Vec<&str>) {
    // This function deals with argument replacements.
    // \$1 -> first argument, \$<N> -> Nth argument,
    // \$* -> all arguments as one string,
    // \$@ -> all arguments as individual arguments.
    // .. also, braced equivalents (\${1}, \${*}, ...).
    //
    // This does not behave exactly like the shell, but tries to get as
    // close as possible.
    //
    // Differences so far:
    // - \$* / \${*} / \$@ / \${@} only work if they are indivdual arguments,
    //   not within another. So 'arg1 \$* arg2' works, '"some \$* arg1" arg2' won't.
    // - whereas in shell, '$*' and '$@' are equivalent and only '"$@"' keeps the individual args,
    //   laze uses star variant for single string, at variant for individual args.
    //   This is because `shell_words::split()` eats the double quotes.

    // lazily create the replacement for '\$*'
    let args_joined = std::cell::LazyCell::new(|| args.iter().join(" "));

    // These two create a helper `VariableMap` to be used by `substitute()`.
    let args_ = LocalVec::new(args);
    let variables = IgnoreMissing::new(&args_);

    // here we iterate the arguments, and:
    // 1. `\$@` / `\${@}` are replaced by multiple individual args.
    // 2. `\$*` / `\${*}` are replaced by the concatenated args
    // 3. all elements run through `substitute()`, substituting the numbered args.
    *cmd = cmd
        .iter()
        .flat_map(|arg| match arg.as_str() {
            "\"$@\"" | "\"${@}\"" | "$@" | "${@}" => args.clone(),
            _ => vec![arg.as_str()],
        })
        .map(|s| match s {
            "$*" | "${*}" | "$@" | "${@}" => args_joined.clone(),
            _ => substitute(s, &variables)
                .with_context(|| format!("substituting '{s}'"))
                .unwrap(),
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();
}
