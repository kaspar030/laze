use std::path::Path;

use anyhow::{Error, Result};
use thiserror::Error;

use crate::nested_env;
use crate::serde_bool_helpers::{default_as_false, default_as_true};
use crate::IGNORE_SIGINT;

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
            command.current_dir(start_dir).arg("-c");

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
                IGNORE_SIGINT.store(true, std::sync::atomic::Ordering::SeqCst);
            }

            // run command, wait for status
            let status = command.status().expect("executing command");

            if self.ignore_ctrl_c {
                IGNORE_SIGINT.store(false, std::sync::atomic::Ordering::SeqCst);
            }

            if !status.success() {
                return Err(anyhow!("task failed"));
            }
        }
        Ok(())
    }

    fn _with_env(&self, env: &im::HashMap<&String, String>, do_eval: bool) -> Result<Task, Error> {
        Ok(Task {
            cmd: self
                .cmd
                .iter()
                .map(|cmd| {
                    if do_eval {
                        nested_env::expand_eval(cmd, env, nested_env::IfMissing::Ignore)
                    } else {
                        nested_env::expand(cmd, env, nested_env::IfMissing::Ignore)
                    }
                })
                .collect::<Result<Vec<String>, _>>()?,
            export: if do_eval {
                self.expand_export(env)
            } else {
                self.export.clone()
            },
            ..(*self).clone()
        })
    }

    pub fn with_env(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        self._with_env(env, false)
    }

    pub fn with_env_eval(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        self._with_env(env, true)
    }

    fn expand_export(&self, env: &im::HashMap<&String, String>) -> Option<Vec<VarExportSpec>> {
        VarExportSpec::expand(self.export.as_ref(), env)
    }
}
