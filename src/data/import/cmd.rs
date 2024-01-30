use anyhow::Context as _;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Command {
    name: Option<String>,
    command: String,
    dldir: Option<String>,
}

impl super::Import for Command {
    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    fn get_dldir(&self) -> Option<&String> {
        self.dldir.as_ref()
    }

    fn handle<T: AsRef<camino::Utf8Path>>(
        &self,
        build_dir: T,
    ) -> Result<camino::Utf8PathBuf, anyhow::Error> {
        let path = self.get_path(&build_dir)?;

        std::fs::create_dir_all(&path).with_context(|| format!("creating {path}"))?;

        // Run the command
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .current_dir(&path)
            .status()
            .with_context(|| {
                format!(
                    "executing command \"{}\" in path \"{}\"",
                    &self.command, &path
                )
            })?;

        if !status.success() {
            return Err(anyhow!(format!(
                "executing command \"{}\" in path \"{}\" failed with exit code {}",
                &self.command, &path, status
            )));
        }

        super::get_lazefile(&path)
    }
}
