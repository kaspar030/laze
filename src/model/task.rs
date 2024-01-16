use std::collections::HashMap;
use std::path::Path;

use anyhow::{Error, Result};

use crate::nested_env;
use crate::serde_bool_helpers::{default_as_false, default_as_true};
use crate::utils::StringOrMapString;
use crate::IGNORE_SIGINT;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Task {
    cmd: Vec<String>,
    pub help: Option<String>,
    pub required_vars: Option<Vec<String>>,
    pub export: Option<Vec<StringOrMapString>>,
    #[serde(default = "default_as_true")]
    build: bool,
    #[serde(default = "default_as_false")]
    ignore_ctrl_c: bool,
}

impl Task {
    pub fn build_app(&self) -> bool {
        self.build
    }

    pub fn execute(
        &self,
        start_dir: &Path,
        args: Option<Vec<&str>>,
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
                    if let StringOrMapString::Map(m) = entry {
                        for (key, val) in m {
                            command.env(key, val);
                        }
                    }
                }
            }

            if let Some(args) = &args {
                command.arg(cmd.clone() + " " + &join(args).to_owned());
            } else {
                command.arg(cmd);
            }

            if self.ignore_ctrl_c {
                IGNORE_SIGINT.store(true, std::sync::atomic::Ordering::SeqCst);
            }

            // run command, wait for status
            command.status().expect("command exited with error code");

            if self.ignore_ctrl_c {
                IGNORE_SIGINT.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
        Ok(())
    }

    fn _with_env(&self, env: &im::HashMap<&String, String>, do_eval: bool) -> Result<Task, Error> {
        Ok(Task {
            help: self.help.clone(),
            build: self.build,
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
            ignore_ctrl_c: self.ignore_ctrl_c,
            required_vars: self.required_vars.clone(),
            export: if do_eval {
                self.expand_export(env)
            } else {
                self.export.clone()
            },
        })
    }

    pub fn with_env(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        self._with_env(env, false)
    }

    pub fn with_env_eval(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        self._with_env(env, true)
    }

    fn expand_export(&self, env: &im::HashMap<&String, String>) -> Option<Vec<StringOrMapString>> {
        // TODO: yeah, this is not a beauty ...
        // what this does is, apply the env to the format as given by "export:"
        //
        // e.g., assuming `FOO=value` and FOOBAR=`other_value`:
        // ```yaml
        //
        // export:
        //   - FOO
        //   - BAR: bar
        //   - FOOBAR: ${foobar}
        // ```
        //
        // ... to export `FOO=value`, `BAR=bar` and `FOOBAR=other_value`.

        self.export.as_ref().map(|exports| exports
                    .iter()
                    .map(|entry| match entry {
                        StringOrMapString::String(s) => {
                            StringOrMapString::Map(HashMap::from_iter([(
                                s.clone(),
                                nested_env::expand_eval(
                                    format!("${{{s}}}"),
                                    env,
                                    nested_env::IfMissing::Empty,
                                )
                                .unwrap(),
                            )]))
                        }
                        StringOrMapString::Map(m) => {
                            StringOrMapString::Map(HashMap::from_iter(m.iter().map(|(k, v)| {
                                (
                                    k.clone(),
                                    nested_env::expand_eval(v, env, nested_env::IfMissing::Empty)
                                        .unwrap(),
                                )
                            })))
                        }
                    })
                    .collect())
    }
}
