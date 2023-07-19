use std::path::Path;

use anyhow::{Error, Result};

use crate::nested_env;
use crate::serde_bool_helpers::{default_as_false, default_as_true};
use crate::IGNORE_SIGINT;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Task {
    cmd: Vec<String>,
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

    pub fn with_env(&self, env: &im::HashMap<&String, String>) -> Result<Task, Error> {
        Ok(Task {
            build: self.build,
            cmd: self
                .cmd
                .iter()
                .map(|cmd| nested_env::expand(cmd, env, nested_env::IfMissing::Ignore))
                .collect::<Result<Vec<String>, _>>()?,
            ignore_ctrl_c: self.ignore_ctrl_c,
        })
    }
}
