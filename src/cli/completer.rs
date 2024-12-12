use std::{
    env,
    sync::{LazyLock, Mutex},
};

use camino::Utf8PathBuf;
use clap_complete::CompletionCandidate;

use crate::model::ContextBag;

static STATE: LazyLock<Mutex<CompleterState>> = LazyLock::new(|| Mutex::new(CompleterState::new()));

#[derive(Default)]
struct CompleterState {
    contexts: Option<ContextBag>,
}

impl CompleterState {
    pub fn new() -> Self {
        let cwd = Utf8PathBuf::try_from(env::current_dir().unwrap()).expect("cwd not UTF8");
        let project_root = crate::determine_project_root(&cwd);
        if let Ok((project_root, project_file)) = project_root {
            let build_dir = project_root.join("build");
            let (contexts, _treestate, _stats) =
                crate::data::load(&project_file, &build_dir).unwrap();
            Self {
                contexts: Some(contexts),
            }
        } else {
            Self { contexts: None }
        }
    }

    pub fn builders(&self) -> Vec<CompletionCandidate> {
        if let Some(contexts) = self.contexts.as_ref() {
            contexts
                .builders()
                .map(|builder| {
                    CompletionCandidate::new(&builder.name)
                        .help(builder.help.as_ref().map(|help| help.into()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn apps(&self) -> Vec<CompletionCandidate> {
        if let Some(contexts) = self.contexts.as_ref() {
            contexts
                .modules()
                .filter(|(_, module)| module.is_binary)
                .map(|(name, module)| {
                    CompletionCandidate::new(name)
                        .help(module.help.as_ref().map(|help| help.into()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn modules(&self) -> Vec<CompletionCandidate> {
        if let Some(contexts) = self.contexts.as_ref() {
            contexts
                .modules()
                .map(|(name, module)| {
                    CompletionCandidate::new(name)
                        .help(module.help.as_ref().map(|help| help.into()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn tasks(&self) -> Vec<CompletionCandidate> {
        if let Some(contexts) = self.contexts.as_ref() {
            contexts
                .modules()
                .flat_map(|(_name, module)| module.tasks.iter())
                .chain(
                    contexts
                        .contexts
                        .iter()
                        .filter_map(|c| c.tasks.as_ref())
                        .flat_map(|tasks| tasks.iter()),
                )
                .map(|(name, task)| {
                    CompletionCandidate::new(name).help(task.help.as_ref().map(|help| help.into()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

pub fn app_completer() -> Vec<CompletionCandidate> {
    let state = STATE.lock().unwrap();
    state.apps()
}

pub fn builder_completer() -> Vec<CompletionCandidate> {
    let state = STATE.lock().unwrap();
    state.builders()
}

pub fn module_completer() -> Vec<CompletionCandidate> {
    let state = STATE.lock().unwrap();
    state.modules()
}

pub fn task_completer() -> Vec<CompletionCandidate> {
    let state = STATE.lock().unwrap();
    state.tasks()
}
