use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::Env;
use crate::MergeOption;
use crate::{ContextBag, Dependency, Module, Rule, Task};

#[derive(PartialEq, Eq)]
pub struct Context {
    pub name: String,
    pub parent_name: Option<String>,

    pub index: Option<usize>,
    pub parent_index: Option<usize>,
    pub modules: IndexMap<String, Module>,
    pub rules: Option<IndexMap<String, Rule>>,
    pub env: Option<Env>,
    pub select: Option<Vec<Dependency>>,
    pub disable: Option<Vec<String>>,

    pub var_options: Option<im::HashMap<String, MergeOption>>,

    pub tasks: Option<HashMap<String, Task>>,
    pub env_early: Env,
    pub is_builder: bool,
    pub defined_in: Option<PathBuf>,
}

impl Context {
    pub fn new(name: String, parent_name: Option<String>) -> Context {
        Context {
            name,
            parent_name,
            index: None,
            parent_index: None,
            modules: IndexMap::new(),
            select: None,
            disable: None,
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
        let builder_index = builder.index.unwrap();

        Context {
            name,
            parent_name: Some(builder.name.clone()),
            index: None,
            parent_index: Some(builder_index),
            modules: IndexMap::new(),
            select: None,
            disable: None,
            env: None,
            env_early: Env::new(),
            rules: None,
            var_options: None,
            tasks: None,
            is_builder: false,
            defined_in: None,
        }
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
                Some((&self, module))
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
            Some(id) => Some(&bag.contexts[id]).unwrap().count_parents(bag) + 1,
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
        env: &im::HashMap<&String, String>,
    ) -> IndexMap<String, Task> {
        let mut result = IndexMap::new();
        let mut parents = Vec::new();
        self.get_parents(contexts, &mut parents);
        for parent in parents {
            if let Some(tasks) = &parent.tasks {
                for (name, task) in tasks {
                    result.insert(name.clone(), task.with_env(env));
                }
            }
        }
        result
    }

    pub fn collect_disabled_modules(&self, contexts: &ContextBag) -> HashSet<String> {
        let mut result = HashSet::new();
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

    pub fn collect_selected_modules(&self, contexts: &ContextBag) -> Vec<Dependency> {
        let mut result = Vec::new();
        let mut parents = Vec::new();
        self.get_parents(contexts, &mut parents);
        for parent in parents {
            if let Some(select) = &parent.select {
                for entry in select {
                    result.push(entry.clone());
                }
            }
        }
        result
    }

    pub fn apply_early_env(&mut self) {
        if let Some(env) = &self.env {
            self.env = Some(crate::nested_env::expand_env(env, &self.env_early));
        }
    }
}

impl Hash for Context {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
