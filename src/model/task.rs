use std::path::Path;

use anyhow::{Error, Result};
use thiserror::Error;

use crate::nested_env;
use crate::serde_bool_helpers::{default_as_false, default_as_true};
use crate::EXIT_ON_SIGINT;

use super::shared::VarExportSpec;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub cmd: Vec<String>,
    pub help: Option<String>,
    pub required_vars: Option<Vec<String>>,
    pub required_modules: Option<Vec<String>>,
    pub export: Option<Vec<VarExportSpec>>,
    #[serde(default = "default_as_true")]
    pub build: bool,
    #[serde(default = "default_as_false")]
    pub ignore_ctrl_c: bool,
    pub working_directory: Option<String>,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum TaskError {
    #[error("required variable `{var}` not set")]
    RequiredVarMissing { var: String },
    #[error("required module `{module}` not selected")]
    RequiredModuleMissing { module: String },
}

impl Task {
    pub fn build_app(&self) -> bool {
        self.build
    }

    pub fn execute(
        &self,
        start_dir: &Path,
        args: Option<&Vec<&str>>,
        verbose: u8,
    ) -> Result<(), Error> {
        for cmd in &self.cmd {
            use shell_words::join;
            use std::process::Command;
            let mut command = Command::new("sh");

            let cmd = cmd.replace("$$", "$");
            if verbose > 0 {
                command.arg("-x");
            }

            if let Some(working_directory) = &self.working_directory {
                // This includes support for absolute working directories through .join
                command.current_dir(start_dir.join(working_directory));
            } else {
                command.current_dir(start_dir);
            }

            command.arg("-c");

            // handle "export:" (export laze variables to task shell environment)
            if let Some(export) = &self.export {
                for entry in export {
                    let VarExportSpec { variable, content } = entry;
                    if let Some(val) = content {
                        command.env(variable, val);
                    }
                }
            }

            if let Some(args) = args {
                command.arg(cmd.clone() + " " + &join(args).to_owned());
            } else {
                command.arg(cmd);
            }

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

            if !status.success() {
                return Err(anyhow!("task failed"));
            }
        }
        Ok(())
    }

    fn _with_env(&self, env: &im::HashMap<&String, String>, do_eval: bool) -> Result<Task, Error> {
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
            working_directory: self.working_directory.as_ref().map(expand).transpose()?,
            ..(*self).clone()
        })
    }

    /// This is called early when loading the yaml files.
    /// It will not evaluate expressions, and pass-through variables that are not
    /// found in `env`.
    pub fn with_env(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        self._with_env(env, false)
    }

    /// This is called to generate the final task.
    /// It will evaluate expressions, and variables that are not
    /// found in `env` will be replaced with the empty string.
    pub fn with_env_eval(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        self._with_env(env, true)
    }

    fn expand_export(&self, env: &im::HashMap<&String, String>) -> Option<Vec<VarExportSpec>> {
        VarExportSpec::expand(self.export.as_ref(), env)
    }
}
