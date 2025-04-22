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
    keep_going: usize,
    project_root: &Path,
) -> Result<(Vec<RunTaskResult<'a>>, usize), Error>
where
    I: Iterator<Item = &'a (&'a BuildInfo, &'a Task)>,
{
    let mut results = Vec::new();
    let mut errors = 0;

    for (build, task) in tasks {
        if verbose > 0 {
            println!(
                "laze: executing task {} for builder {} bin {}",
                task_name, build.builder, build.binary,
            );
        }

        let all_tasks = &build.tasks;

        let result = task.execute(project_root, args, verbose, all_tasks);
        let is_error = result.is_err();

        results.push(RunTaskResult {
            build,
            task,
            result,
        });

        if is_error {
            errors += 1;
            if keep_going > 0 && errors >= keep_going {
                break;
            }
        }
    }

    Ok((results, errors))
}
