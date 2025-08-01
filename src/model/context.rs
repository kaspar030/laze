use anyhow::{Context as _, Error};
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use camino::Utf8PathBuf;

use crate::nested_env::EnvMap;
use crate::Env;
use crate::MergeOption;
use crate::{ContextBag, Module, Rule, Task, TaskError};

#[derive(Eq)]
pub struct Context {
    pub name: String,
    pub parent_name: Option<String>,

    pub help: Option<String>,

    pub index: Option<usize>,
    pub parent_index: Option<usize>,
    pub modules: IndexMap<String, Module>,
    pub rules: Option<IndexMap<String, Rule>>,
    pub env: Option<Env>,
    // TODO(context-early-disables)
    pub disable: Option<Vec<String>>,

    // map of providables that are provided in this context or its parents
    pub provided: Option<im::HashMap<String, IndexSet<String>>>,

    pub var_options: Option<im::HashMap<String, MergeOption>>,

    pub tasks: Option<HashMap<String, Task>>,
    pub env_early: Env,
    pub is_builder: bool,
    pub defined_in: Option<Utf8PathBuf>,
}

impl Context {
    pub fn new(name: String, parent_name: Option<String>) -> Context {
        Context {
            name,
            parent_name,
            help: None,
            index: None,
            parent_index: None,
            modules: IndexMap::new(),
            disable: None,
            provided: None,
            env: None,
            env_early: Env::new(),
            rules: None,
            var_options: None,
            tasks: None,
            is_builder: false,
            defined_in: None,
        }
    }

    pub fn new_build_context(name: String, builder: &Context) -> Context {
        let mut context = Context::new(name, Some(builder.name.clone()));
        context.parent_index = Some(builder.index.unwrap());
        context
    }

    pub fn get_parent<'a>(&self, bag: &'a ContextBag) -> Option<&'a Context> {
        match self.parent_index {
            Some(id) => Some(&bag.contexts[id]),
            None => None,
        }
    }

    fn get_parents<'a>(&'a self, bag: &'a ContextBag, result: &mut Vec<&'a Context>) {
        if let Some(parent) = self.get_parent(bag) {
            parent.get_parents(bag, result);
        }
        result.push(self);
    }

    /// Creates an iterator over a context and its parents, starting
    /// with the context.
    pub(crate) fn context_iter<'a>(&'a self, bag: &'a ContextBag) -> ParentIterator<'a> {
        ParentIterator {
            bag,
            next_context: Some(self),
        }
    }

    pub fn resolve_module<'a: 'b, 'b>(
        &'a self,
        module_name: &String,
        bag: &'b ContextBag,
    ) -> Option<(&'b Context, &'b Module)> {
        //println!("resolving module {} in {}...", module_name, self.name);
        let module = self.modules.get(module_name);
        match module {
            Some(module) => {
                //println!("found module {} in {}.", module_name, self.name);
                Some((self, module))
            }
            None => match self.parent_index {
                Some(id) => {
                    let parent = &bag.contexts[id];
                    //println!("descending");
                    parent.resolve_module(module_name, bag)
                }
                None => {
                    //println!("no more parents, module not found");
                    None
                }
            },
        }
    }

    pub fn count_parents(&self, bag: &ContextBag) -> usize {
        match self.parent_index {
            Some(id) => &bag.contexts[id].count_parents(bag) + 1,
            None => 0,
        }
    }

    // This function collects all rules of a given context and all its parents.
    // The resulting indexmap is indexed by the "in_" field (which should be an extension),
    // or if absent, by name
    pub fn collect_rules<'a>(
        &'a self,
        contexts: &'a ContextBag,
        result: &'a mut IndexMap<String, &'a Rule>,
    ) -> &'a mut IndexMap<String, &'a Rule> {
        //println!("collecting rules for {}", self.name);
        let mut parents = Vec::new();
        self.get_parents(contexts, &mut parents);
        for parent in parents {
            //println!("merging parent {}", parent.name);
            if let Some(rules) = &parent.rules {
                for (_, rule) in rules {
                    //println!("rule {}", rule.name);
                    if let Some(rule_in) = rule.in_.as_ref() {
                        result.insert(rule_in.clone(), rule);
                    } else {
                        result.insert(rule.name.clone(), rule);
                    }
                }
            }
        }
        result
    }

    pub fn collect_tasks(
        &self,
        contexts: &ContextBag,
        env: &EnvMap,
        modules: &IndexMap<&String, (&Module, Env, std::option::Option<IndexSet<&Module>>)>,
    ) -> Result<IndexMap<String, Result<Task, TaskError>>, Error> {
        let mut result = IndexMap::new();
        let mut parents = Vec::new();
        self.get_parents(contexts, &mut parents);
        for parent in parents {
            if let Some(tasks) = &parent.tasks {
                for (name, task) in tasks {
                    if !task_handle_required_vars(task, env, &mut result, name) {
                        continue;
                    }

                    if !task_handle_required_modules(task, modules, &mut result, name) {
                        continue;
                    }
                    result.insert(
                        name.clone(),
                        Ok(task
                            .with_env_eval(env)
                            .with_context(|| format!("task \"{}\"", name))?),
                    );
                }
            }

            // module tasks
            for (_module_name, (module, _module_env, _)) in modules {
                for (name, task) in &module.tasks {
                    if !task_handle_required_vars(task, env, &mut result, name) {
                        continue;
                    }
                    if !task_handle_required_modules(task, modules, &mut result, name) {
                        continue;
                    }
                    result.insert(
                        name.clone(),
                        Ok(task
                            .with_env_eval(env)
                            .with_context(|| format!("task \"{}\"", name))?),
                    );
                }
            }
        }
        Ok(result)
    }

    pub fn collect_disabled_modules(&self, contexts: &ContextBag) -> IndexSet<String> {
        let mut result = IndexSet::new();
        let mut parents = Vec::new();
        self.get_parents(contexts, &mut parents);
        for parent in parents {
            if let Some(disable) = &parent.disable {
                for entry in disable {
                    result.insert(entry.clone());
                }
            }
        }
        result
    }

    pub fn apply_early_env(&mut self) -> Result<(), Error> {
        if let Some(env) = &mut self.env {
            env.expand(&self.env_early)?;
        }
        Ok(())
    }

    pub fn module_name(&self) -> String {
        Self::module_name_for(&self.name)
    }

    pub(crate) fn module_name_for(context_name: &str) -> String {
        format!("context::{}", context_name)
    }

    pub(crate) fn new_default() -> Context {
        let mut default = Context::new("default".to_string(), None);
        let default_module =
            Module::new("context::default".to_string(), Some("default".to_string()));

        default
            .modules
            .insert("context::default".to_string(), default_module);

        default
    }
}

fn task_handle_required_modules(
    task: &Task,
    modules: &IndexMap<&String, (&Module, Env, Option<IndexSet<&Module>>)>,
    result: &mut IndexMap<String, Result<Task, TaskError>>,
    name: &str,
) -> bool {
    if let Some(required_modules) = &task.required_modules {
        for module in required_modules {
            if !modules.contains_key(module) {
                result.insert(
                    name.to_string(),
                    Err(TaskError::RequiredModuleMissing {
                        module: module.clone(),
                    }),
                );
                return false;
            }
        }
    }
    true
}

fn task_handle_required_vars(
    task: &Task,
    env: &EnvMap,
    result: &mut IndexMap<String, Result<Task, TaskError>>,
    name: &str,
) -> bool {
    if let Some(required_vars) = &task.required_vars {
        for var in required_vars {
            if !env.contains_key(var.as_str()) {
                result.insert(
                    name.to_string(),
                    Err(TaskError::RequiredVarMissing { var: var.clone() }),
                );
                return false;
            }
        }
    }
    true
}

impl Hash for Context {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub struct ParentIterator<'a> {
    bag: &'a ContextBag,
    next_context: Option<&'a Context>,
}

impl<'a> Iterator for ParentIterator<'a> {
    type Item = &'a Context;
    fn next(&mut self) -> Option<&'a Context> {
        let res = self.next_context;
        self.next_context = self.next_context.and_then(|c| c.get_parent(self.bag));
        res
    }
}
