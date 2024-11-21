use std::path::Path;

use anyhow::Error;

use crate::generate::BuildInfo;
use crate::model::Task;

#[derive(Debug)]
pub struct RunTaskResult<'a> {
    pub build: &'a BuildInfo,
    #[allow(dead_code)]
    pub task: &'a Task,
    pub result: Result<(), Error>,
}

pub fn run_tasks<'a, I>(
    task_name: &str,
    tasks: I,
    args: std::option::Option<&Vec<&str>>,
    verbose: u8,
    project_root: &Path,
) -> Result<(Vec<RunTaskResult<'a>>, usize), Error>
where
    I: Iterator<Item = &'a (&'a BuildInfo, &'a Task)>,
{
    let mut results = Vec::new();
    let mut errors = 0;

    for (build, task) in tasks {
        println!(
            "laze: executing task {} for builder {} bin {}",
            task_name, build.builder, build.binary,
        );

        let result = task.execute(project_root, args, verbose);
        if result.is_err() {
            errors += 1;
        }

        let result = RunTaskResult {
            build,
            task,
            result,
        };
        results.push(result);
    }

    Ok((results, errors))
}
