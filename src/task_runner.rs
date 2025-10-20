use std::path::Path;

use anyhow::Error;
use log::debug;

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
    keep_going: usize,
    project_root: &Path,
) -> Result<(Vec<RunTaskResult<'a>>, usize), Error>
where
    I: Iterator<Item = &'a (&'a BuildInfo, &'a Task)>,
{
    let mut results = Vec::new();
    let mut errors = 0;

    for (build, task) in tasks {
        debug!(
            "laze: executing task {} for builder {} bin {}",
            task_name, build.builder, build.binary,
        );

        let all_tasks = &build.tasks;

        let parent_export = im::Vector::new();
        let result = task.execute(project_root, args, all_tasks, &parent_export);
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
