#[macro_use]
extern crate anyhow;
extern crate clap;

#[macro_use]
extern crate simple_error;

#[macro_use]
extern crate derive_builder;

extern crate pathdiff;

use std::collections::{HashMap, HashSet};
//use std::error::Error;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{iter, iter::Filter, slice::Iter};

#[macro_use]
extern crate serde_derive;

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use anyhow::{Context as _, Error, Result};
use clap::{crate_version, App, AppSettings, Arg, SubCommand};

mod nested_env;
use nested_env::{Env, IfMissing, MergeOption};

mod data;
use data::load;

mod ninja;
use ninja::{
    NinjaBuildBuilder, NinjaCmdBuilder, NinjaRule, NinjaRuleBuilder, NinjaRuleDeps, NinjaWriter,
};

#[derive(PartialEq, Eq)]
pub struct Context {
    pub name: String,
    pub parent_name: Option<String>,

    pub index: Option<usize>,
    pub parent_index: Option<usize>,
    pub modules: IndexMap<String, Module>,
    pub rules: Option<IndexMap<String, Rule>>,
    pub env: Option<Env>,
    pub disable: Option<Vec<String>>,

    pub var_options: Option<HashMap<String, MergeOption>>,

    pub tasks: Option<HashMap<String, Task>>,
    pub env_early: Env,
    pub is_builder: bool,
    pub defined_in: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Task {
    cmd: Vec<String>,
}

impl Task {
    pub fn execute(&self, start_dir: &Path, args: Option<Vec<&str>>) -> Result<(), Error> {
        for cmd in &self.cmd {
            use shell_words::join;
            use std::process::Command;
            let mut command = Command::new("sh");
            command.current_dir(start_dir).arg("-c");

            if let Some(args) = &args {
                command.arg(cmd.clone() + " " + &join(args).to_owned());
            } else {
                command.arg(cmd);
            }

            command.status().expect("command exited with error code");
        }
        Ok(())
    }

    pub fn with_env(&self, env: &HashMap<&String, String>) -> Task {
        Task {
            cmd: self
                .cmd
                .iter()
                .map(|cmd| nested_env::expand(cmd, env, nested_env::IfMissing::Ignore).unwrap())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Dependency {
    Hard(String),
    Soft(String),
    //Conflict(String),
    IfThenHard(String, String),
    IfThenSoft(String, String),
    //IfThenConflict(String, String),
}

impl Dependency {
    pub fn get_name(&self) -> &String {
        match self {
            Dependency::Hard(name) => name,
            Dependency::Soft(name) => name,
            Dependency::IfThenHard(_, name) => name,
            Dependency::IfThenSoft(_, name) => name,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Module {
    name: String,
    context_name: String,

    selects: Vec<Dependency>,
    imports: Vec<Dependency>,
    disable: Option<Vec<String>>,

    blocklist: Option<Vec<String>>,
    allowlist: Option<Vec<String>>,

    sources: Vec<String>,
    sources_optional: Option<IndexMap<String, Vec<String>>>,

    env_local: Env,
    env_export: Env,
    env_global: Env,
    env_early: Env,

    context_id: Option<usize>,
    defined_in: Option<PathBuf>,
    relpath: Option<PathBuf>,
    srcdir: Option<PathBuf>,
    is_binary: bool,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Rule {
    name: String,
    cmd: String,

    #[serde(rename = "in")]
    in_: Option<String>,
    out: Option<String>,
    context: Option<String>,
    options: Option<HashMap<String, String>>,
    gcc_deps: Option<String>,
}

impl Context {
    pub fn new(name: String, parent_name: Option<String>) -> Context {
        Context {
            name,
            parent_name,
            index: None,
            parent_index: None,
            modules: IndexMap::new(),
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
                    }
                }
            }
        }
        result
    }

    pub fn collect_tasks(
        &self,
        contexts: &ContextBag,
        env: &HashMap<&String, String>,
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

    pub fn apply_early_env(&mut self) {
        if let Some(env) = &self.env {
            self.env = Some(nested_env::expand_env(env, &self.env_early));
        }
    }
}

impl Hash for Context {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

pub struct ContextBag {
    contexts: Vec<Context>,
    context_map: HashMap<String, usize>,
    //    module_map: HashMap<String, usize>,
}

pub enum IsAncestor {
    No,
    Yes(usize, usize),
}

pub enum BlockAllow {
    Allowed,
    AllowedBy(usize),
    Blocked,
    BlockedBy(usize),
}

impl BlockAllow {
    pub fn allow(index: usize, depth: usize) -> BlockAllow {
        match depth {
            0 => BlockAllow::Allowed,
            _ => BlockAllow::AllowedBy(index),
        }
    }
    pub fn block(index: usize, depth: usize) -> BlockAllow {
        match depth {
            0 => BlockAllow::Blocked,
            _ => BlockAllow::BlockedBy(index),
        }
    }
}

impl ContextBag {
    pub fn new() -> ContextBag {
        ContextBag {
            contexts: Vec::new(),
            context_map: HashMap::new(),
            //module_map: HashMap::new(),
        }
    }

    pub fn get_by_name(&self, name: &String) -> Option<&Context> {
        let id = self.context_map.get(name);
        match id {
            Some(id) => Some(&self.contexts[*id]),
            None => None,
        }
    }

    pub fn get_by_name_mut(&mut self, name: &String) -> Option<&mut Context> {
        let id = self.context_map.get(name);
        match id {
            Some(id) => Some(&mut self.contexts[*id]),
            None => None,
        }
    }

    pub fn finalize(&mut self) {
        /* ensure there's a "default" context */
        if let None = self.get_by_name(&"default".to_string()) {
            self.add_context(Context::new("default".to_string(), None))
                .unwrap();
        }

        /* set the "parent" index of each context that sets a "parent_name" */
        for context in &mut self.contexts {
            if let Some(parent_name) = &context.parent_name {
                let parent = self.context_map.get(&parent_name.clone()).unwrap();
                context.parent_index = Some(*parent);
            }
        }

        /* merge environments of parent context, recursively. to do that,
         * we need to ensure that we process the contexts in an order so that each context is
         * processed after all its parents have been processed.
         * This can be done by topologically sorting them by the total numbers of parents.
         * Also, collect var_options for each builder.
         */

        /* 1. sort by number of parents (ascending) */
        let mut sorted_by_numparents: Vec<_> = self
            .contexts
            .iter()
            .enumerate()
            .map(|(n, context)| (n, context.count_parents(self)))
            .collect();

        sorted_by_numparents.sort_by(|a, b| a.1.cmp(&b.1));

        /* 2. merge ordered by number of parents (ascending) */
        for (n, m) in &sorted_by_numparents {
            let n = *n;
            let m = *m;
            if m == 0 {
                continue;
            }

            let context = &self.contexts[n];
            let context_env = context.env.as_ref();
            let parent_env = &self.contexts[context.parent_index.unwrap()].env.as_ref();

            if let Some(parent_env) = parent_env {
                let mut env = Env::new();
                nested_env::merge(&mut env, &parent_env);
                if let Some(context_env) = context_env {
                    nested_env::merge(&mut env, &context_env);
                }
                if context.is_builder {
                    env.insert(
                        "builder".to_string(),
                        nested_env::EnvKey::Single(context.name.clone()),
                    );
                }
                let context = &mut self.contexts[n];
                context.env = Some(env);
            }
        }
        for (n, m) in sorted_by_numparents {
            if m == 0 {
                continue;
            }
            // this looks complicated...
            // the idea is,
            // if a parent has var_opts,
            //    if a context has var_opts
            //       merge parent in context options
            //    else
            //       clone parent options

            let context = &self.contexts[n];
            let parent_var_ops = &self.contexts[context.parent_index.unwrap()].var_options;
            let combined_var_opts = if let Some(parent_var_ops) = parent_var_ops {
                if let Some(context_var_opts) = &context.var_options {
                    Some(
                        parent_var_ops
                            .into_iter()
                            .chain(context_var_opts)
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    )
                } else {
                    Some(parent_var_ops.clone())
                }
            } else {
                None
            };
            let mut context = &mut self.contexts[n];
            if let None = &context.var_options {
                context.var_options = combined_var_opts;
            }
        }
    }

    pub fn add_context_or_builder(
        &mut self,
        mut context: Context,
        is_builder: bool,
    ) -> Result<&mut Context, Error> {
        if self.context_map.contains_key(&context.name) {
            return Err(anyhow!("context name already used"));
        }

        let last = self.contexts.len();

        context.is_builder = is_builder;
        context.index = Some(last);

        self.context_map.insert(context.name.clone(), last);
        self.contexts.push(context);

        Ok(&mut self.contexts[last])
    }

    pub fn add_context(&mut self, context: Context) -> Result<&mut Context, Error> {
        self.add_context_or_builder(context, false)
    }

    pub fn add_builder(&mut self, context: Context) -> Result<&mut Context, Error> {
        self.add_context_or_builder(context, true)
    }

    pub fn add_module(&mut self, mut module: Module) -> Result<(), Error> {
        let context_id = self.context_map.get(&module.context_name);
        let context_id = match context_id {
            Some(id) => id,
            None => return Err(anyhow!("unknown context")),
        };
        let context = &mut self.contexts[*context_id];
        module.context_id = Some(*context_id);
        context.modules.insert(module.name.clone(), module);
        Ok(())
    }

    pub fn builders(&self) -> Filter<Iter<Context>, fn(&&Context) -> bool> {
        self.contexts.iter().filter(|&x| x.is_builder)
    }

    pub fn builders_vec(&self) -> Vec<&Context> {
        self.builders().collect()
    }

    pub fn builders_by_name(&self, names: &IndexSet<&str>) -> Vec<&Context> {
        let mut res = Vec::new();
        for builder in self.builders() {
            if names.contains(&builder.name[..]) {
                res.push(builder);
            }
        }
        res
    }

    pub fn print(&self) {
        for context in &self.contexts {
            let parent_name = match context.parent_index {
                Some(index) => self.contexts[index].name.clone(),
                None => "none".to_string(),
            };

            println!("context: {} parent: {}", context.name, parent_name);
            for (_, module) in &context.modules {
                println!("        {}", module.name);
            }
        }
    }

    pub fn context_by_id(&self, context_id: usize) -> &Context {
        &self.contexts[context_id]
    }

    /// returns true if context_id is parent of other_context_id
    pub fn is_ancestor(
        &self,
        context_id: usize,
        other_context_id: usize,
        depth: usize,
    ) -> IsAncestor {
        if context_id == other_context_id {
            return IsAncestor::Yes(context_id, depth);
        }
        let context = self.context_by_id(other_context_id);
        match context.parent_index {
            Some(id) => self.is_ancestor(context_id, id, depth + 1),
            None => IsAncestor::No,
        }
    }

    fn is_ancestor_in_list(&self, context: &Context, list: &Vec<String>) -> IsAncestor {
        for context_name in list {
            if let Some(listed_context) = self.get_by_name(context_name) {
                match self.is_ancestor(listed_context.index.unwrap(), context.index.unwrap(), 0) {
                    IsAncestor::No => continue,
                    IsAncestor::Yes(index, depth) => return IsAncestor::Yes(index, depth),
                }
            }
        }
        IsAncestor::No
    }

    fn is_allowed(
        &self,
        context: &Context,
        blocklist: &Option<Vec<String>>,
        allowlist: &Option<Vec<String>>,
    ) -> BlockAllow {
        let allowlist_entry = match allowlist {
            Some(list) => self.is_ancestor_in_list(context, list),
            None => IsAncestor::No,
        };
        let blocklist_entry = match blocklist {
            Some(list) => self.is_ancestor_in_list(context, list),
            None => IsAncestor::No,
        };

        if let Some(_) = allowlist {
            if let Some(_) = blocklist {
                if let IsAncestor::Yes(allow_index, allow_depth) = allowlist_entry {
                    if let IsAncestor::Yes(block_index, block_depth) = blocklist_entry {
                        if allow_depth > block_depth {
                            return BlockAllow::block(block_index, block_depth);
                        }
                    }
                    return BlockAllow::allow(allow_index, allow_depth);
                } else {
                    if let IsAncestor::Yes(block_index, block_depth) = blocklist_entry {
                        return BlockAllow::block(block_index, block_depth);
                    }
                }
            } else {
                if let IsAncestor::No = allowlist_entry {
                    return BlockAllow::Blocked;
                }
            }
        } else {
            if let Some(_) = blocklist {
                if let IsAncestor::Yes(block_index, block_depth) = blocklist_entry {
                    return BlockAllow::block(block_index, block_depth);
                }
            }
        }

        BlockAllow::Allowed
    }
}

// impl fmt::Display for Context {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         for context in &self.contexts {
//             let parent_name = match context.parent {
//                 Some(index) => self.contexts[index].name.clone(),
//                 None => "none".to_string(),
//             };

//             println!("context: {} parent: {}", context.name, parent_name);
//         }
//     }
// }

impl Module {
    fn new(name: String, context_name: Option<String>) -> Module {
        Module {
            name,
            context_name: context_name.unwrap_or_else(|| "default".to_string()),
            selects: Vec::new(),
            imports: Vec::new(),
            disable: None,
            // exports: Vec::new(),
            sources: Vec::new(),
            sources_optional: None,
            env_local: Env::new(),
            env_export: Env::new(),
            env_global: Env::new(),
            env_early: Env::new(),
            context_id: None,
            is_binary: false,
            defined_in: None,
            relpath: None,
            srcdir: None,
            blocklist: None,
            allowlist: None,
        }
    }

    fn from(defaults: &Module, name: String, context_name: Option<String>) -> Module {
        Module {
            name,
            context_name: context_name.unwrap_or_else(|| defaults.context_name.clone()),
            selects: defaults.selects.clone(),
            imports: defaults.imports.clone(),
            disable: defaults.disable.clone(),
            // exports: Vec::new(),
            sources: defaults.sources.clone(),
            sources_optional: defaults.sources_optional.clone(),
            env_local: defaults.env_local.clone(),
            env_export: defaults.env_export.clone(),
            env_global: defaults.env_global.clone(),
            env_early: Env::new(),
            context_id: None,
            is_binary: false,
            defined_in: None,
            relpath: None,
            blocklist: defaults.blocklist.clone(),
            allowlist: defaults.blocklist.clone(),
            srcdir: match &defaults.srcdir {
                Some(dir) => Some(dir.clone()),
                None => None,
            },
        }
    }

    //fn can_build_for(&self, context: &Context, contexts: &ContextBag) -> bool {
    //    contexts.context_is_in(self.context_id.unwrap(), context.index.unwrap())
    //}
    //
    fn get_imports_recursive<'a>(
        &'a self,
        modules: &IndexMap<&String, &'a Module>,
        seen: Option<&mut HashSet<&'a String>>,
    ) -> Vec<&'a Module> {
        let mut result = Vec::new();

        let mut newseen = HashSet::new();
        let seen = match seen {
            Some(seen) => seen,
            None => &mut newseen,
        };

        let module = self;
        if seen.contains(&self.name) {
            return Vec::new();
        }
        seen.insert(&self.name);

        for dep in &module.imports {
            let dep_name = match dep {
                Dependency::Hard(name) => name,
                Dependency::Soft(name) => name,
                Dependency::IfThenHard(other, name) => {
                    if modules.contains_key(other) {
                        name
                    } else {
                        continue;
                    }
                }
                Dependency::IfThenSoft(other, name) => {
                    if modules.contains_key(other) {
                        name
                    } else {
                        continue;
                    }
                }
            };

            if let Some(other_module) = modules.get(&dep_name) {
                let mut other_deps = other_module.get_imports_recursive(modules, Some(seen));
                result.append(&mut other_deps);
            }
        }

        result.push(self);

        result
    }

    fn build_env(&self, global_env: &Env, modules: &IndexMap<&String, &Module>) -> Env {
        /* start with a fresh env */
        let mut module_env = Env::new();
        /* merge in the global build context env */
        nested_env::merge(&mut module_env, global_env);

        /* from each (recursive) import ... */
        let deps = self.get_imports_recursive(&modules, None);
        for dep in &deps {
            /* merge that dependency's exported env */
            nested_env::merge(&mut module_env, &dep.env_export);

            //
            let notify_list = module_env
                .entry("notify".into())
                .or_insert_with(|| nested_env::EnvKey::List(vec![]));

            match notify_list {
                nested_env::EnvKey::Single(_) => panic!("unexpected notify value"),
                nested_env::EnvKey::List(list) => list.push(dep.create_module_define()),
            }
        }

        /* finally, merge the module's local env */
        nested_env::merge(&mut module_env, &self.env_local);

        module_env
    }

    fn create_module_define(&self) -> String {
        self.name
            .chars()
            .map(|x| match x {
                'a'..='z' => x.to_ascii_uppercase(),
                '/' => '_',
                '.' => '_',
                '-' => '_',
                _ => x,
            })
            .collect()
    }

    pub fn apply_early_env(&mut self) {
        self.env_local = nested_env::expand_env(&self.env_local, &self.env_early);
        self.env_export = nested_env::expand_env(&self.env_export, &self.env_early);
        self.env_global = nested_env::expand_env(&self.env_global, &self.env_early);
    }
}

impl Hash for Module {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.context_name.hash(state);
    }
}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /* rules are unique per context subtree, so hashing the name is
         * sufficient. */
        self.name.hash(state);
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.context_name[..] {
            "default" => write!(f, "{}", self.name),
            _ => write!(f, "{}:{}", self.context_name, self.name),
        }
    }
}

struct Build<'a> {
    bag: &'a ContextBag,
    binary: &'a Module,
    builder: &'a Context,
    build_context: Context,
    //modules: IndexMap<&'a String, &'a Module>,
}

impl<'a: 'b, 'b> Build<'b> {
    fn new(binary: &'a Module, builder: &'a Context, contexts: &'a ContextBag) -> Build<'b> {
        let mut build = Build {
            bag: contexts,
            binary,
            builder,
            build_context: Context::new_build_context(builder.name.clone(), builder),
        };

        /* fixup name to "$builder_name:$binary_name" */
        build.build_context.name.push_str(&":");
        build.build_context.name.push_str(&binary.name);

        /* collect environment from builder */
        build.build_context.env = Some(Env::new());
        let mut build_env = build.build_context.env.as_mut().unwrap();

        if let Some(builder_env) = &builder.env.as_ref() {
            nested_env::merge(&mut build_env, builder_env);
        }

        /* add "app" variable */
        // TODO: maybe move to module creation
        build_env.insert(
            "app".to_string(),
            nested_env::EnvKey::Single(binary.name.clone()),
        );

        build
    }

    fn resolve_module_deep<'m, 's: 'm>(
        &'s self,
        module: &'m Module,
        module_set: &mut IndexMap<&'m String, &'m Module>,
        disabled_modules: &HashSet<String>,
    ) -> Result<(), Error> {
        let prev_len = module_set.len();

        fn reset(module_set: &mut IndexMap<&String, &Module>, len: usize) {
            while module_set.len() > len {
                module_set.pop();
            }
        }

        module_set.insert(&module.name, module);

        for dep in &module.selects {
            let (dep_name, optional) = match dep {
                Dependency::Hard(name) => (name, false),
                Dependency::Soft(name) => (name, true),
                Dependency::IfThenHard(other, name) => {
                    if module_set.contains_key(other) {
                        (name, false)
                    } else {
                        continue;
                    }
                }
                Dependency::IfThenSoft(other, name) => {
                    if module_set.contains_key(other) {
                        (name, true)
                    } else {
                        continue;
                    }
                }
            };

            if module_set.contains_key(dep_name) {
                continue;
            }

            if disabled_modules.contains(dep_name) {
                if !optional {
                    reset(module_set, prev_len);
                    bail!(
                        "binary {} for builder {}: {} depends on disabled module {}",
                        self.binary.name,
                        self.builder.name,
                        module.name,
                        dep_name
                    );
                } else {
                    continue;
                }
            }

            let (_context, module) = match self.build_context.resolve_module(dep_name, self.bag) {
                Some(x) => x,
                None => {
                    if optional {
                        continue;
                    } else {
                        reset(module_set, prev_len);
                        bail!(
                            "binary {} for builder {}: {} depends on unavailable module {}",
                            self.binary.name,
                            self.builder.name,
                            module.name,
                            dep_name
                        );
                    }
                }
            };

            if let Err(x) = self.resolve_module_deep(module, module_set, disabled_modules) {
                if !optional {
                    reset(module_set, prev_len);
                    return Err(x);
                }
            }
        }
        Ok(())
    }

    fn resolve_selects(
        &self,
        disabled_modules: &HashSet<String>,
    ) -> Result<IndexMap<&String, &Module>, Error> {
        let mut modules = IndexMap::new();

        if let Err(x) = self.resolve_module_deep(&self.binary, &mut modules, disabled_modules) {
            return Err(x);
        }
        Ok(modules)
    }

    //fn configure(&'a mut self) -> Result<&'a mut Build, Box<dyn Error>> {
    //    Ok(self)
    //}

    //    fn configure(&'a mut self) -> Result<&'a mut Build, Box<dyn Error>> {
    //        self.resolve_modules()?;
    //        Ok(self)
    //    }
}

struct BuildInfo {
    tasks: IndexMap<String, Task>,
}

fn generate(
    project_root: &Path,
    start_dir: &Path,
    build_dir: &Path,
    global: bool,
    builders: &IndexSet<&str>,
    apps: &IndexSet<&str>,
) -> Result<Vec<(String, String, BuildInfo)>> {
    let contexts = load(project_root)?;

    std::fs::create_dir_all(&build_dir)?;
    let mut ninja_writer = NinjaWriter::new(build_dir.join("build.ninja").as_path()).unwrap();

    fn configure_build(
        binary: &Module,
        contexts: &ContextBag,
        builder: &Context,
        ninja_writer: &mut NinjaWriter,
        laze_env: &Env,
    ) -> Result<Option<BuildInfo>> {
        if !match contexts.is_allowed(builder, &binary.blocklist, &binary.allowlist) {
            BlockAllow::Allowed => true,
            BlockAllow::Blocked => {
                println!("app {}: builder {} blocklisted", binary, builder.name);
                false
            }
            BlockAllow::BlockedBy(index) => {
                println!(
                    "app {}: parent {} of builder {} blocklisted",
                    binary.name,
                    contexts.context_by_id(index).name,
                    builder.name
                );
                false
            }
            BlockAllow::AllowedBy(index) => {
                println!(
                    "app {}: parent {} of builder {} allowlisted",
                    binary.name,
                    contexts.context_by_id(index).name,
                    builder.name
                );
                true
            }
        } {
            return Ok(None);
        }

        println!("configuring {} for {}", binary.name, builder.name);

        /* create build instance (binary A for builder X) */
        let build = Build::new(binary, builder, contexts);

        /* create initial build context global env.
         * Unfortunately we need to create a copy as we cannot get a mutable
         * reference to build_context.env. */
        let mut global_env = Env::new();
        nested_env::merge(&mut global_env, &laze_env);
        nested_env::merge(&mut global_env, &build.build_context.env.as_ref().unwrap());

        // collect disabled modules from app and build context
        let mut disabled_modules = build.build_context.collect_disabled_modules(&contexts);
        if let Some(disable) = &binary.disable {
            for entry in disable {
                disabled_modules.insert(entry.clone());
            }
        }

        /* resolve all dependency names to specific modules.
         * this also determines if all dependencies are met */
        let modules = match build.resolve_selects(&disabled_modules) {
            Err(e) => {
                println!("error: {}", e);
                return Ok(None);
            }
            Ok(val) => val,
        };

        /* collect build context rules */
        let mut rules = IndexMap::new();
        let rules = build.build_context.collect_rules(&contexts, &mut rules);
        let merge_opts = &builder.var_options;

        /* import global module environments into global build context env */
        for (_, module) in modules.iter().rev() {
            nested_env::merge(&mut global_env, &module.env_global);
        }

        let mut app_builds = Vec::new();

        /* now handle each module */
        for (_, module) in modules.iter() {
            /* build final module env */
            let module_env = module.build_env(&global_env, &modules);

            /* add escaped ${in} and ${out}, create env for the build rules */
            let mut rule_env = Env::new();
            nested_env::merge(&mut rule_env, &module_env);
            let flattened_env =
                nested_env::flatten_with_opts_option(&rule_env, merge_opts.as_ref());
            //println!("{:#?}", builder.var_options);

            let mut module_rules: IndexMap<String, NinjaRule> = IndexMap::new();
            let mut module_builds = Vec::new();

            // add optional sources, if needed
            let mut optional_sources = Vec::new();
            if let Some(optional_sources_map) = &module.sources_optional {
                for (k, v) in optional_sources_map {
                    if modules.contains_key(k) {
                        for entry in v {
                            optional_sources.push(entry.clone());
                        }
                    }
                }
            }

            /* apply rules to sources */
            for source in module.sources.iter().chain(optional_sources.iter()) {
                let ext = Path::new(&source)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap();

                module_rules.entry(ext.into()).or_insert({
                    let rule = match rules.get(ext.into()) {
                        Some(rule) => rule,
                        None => {
                            return Err(anyhow!(
                                "no rule found for \"{}\" of module \"{}\" (from {})",
                                source,
                                module.name,
                                module.defined_in.as_ref().unwrap().to_string_lossy(),
                            ))
                        }
                    };
                    let expanded =
                        nested_env::expand(&rule.cmd, &flattened_env, IfMissing::Empty).unwrap();

                    NinjaRuleBuilder::default()
                        .name(&*rule.name)
                        .description(&*rule.name)
                        .command(&*expanded)
                        .deps(match &rule.gcc_deps {
                            None => NinjaRuleDeps::None,
                            Some(s) => NinjaRuleDeps::GCC(s.clone()),
                        })
                        .build()
                        .unwrap()
                });
            }

            let srcdir = module.defined_in.as_ref().unwrap().parent().unwrap();
            for source in module.sources.iter().chain(optional_sources.iter()) {
                let ext = Path::new(&source)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap();

                let mut srcpath = srcdir.to_path_buf();
                srcpath.push(source);
                let rule = rules.get(ext.into()).unwrap();
                let ninja_rule = module_rules.get(ext.into()).unwrap();
                let out = srcpath.with_extension(&rule.out.as_ref().unwrap());
                module_builds.push((ninja_rule.clone(), srcpath, out));
            }

            app_builds.append(&mut module_builds);
        }

        let mut objects = Vec::new();

        let relpath = binary.relpath.as_ref().unwrap();
        global_env.insert(
            "relpath".to_string(),
            nested_env::EnvKey::Single(String::from(relpath.to_str().unwrap())),
        );

        let (ninja_link_rule, bindir) = {
            let link_rule = match rules.values().find(|rule| rule.name == "LINK") {
                Some(x) => x,
                // returning an error here won't show, just not configure the build
                // None => return Err(anyhow!("missing LINK rule for builder {}", builder.name)),
                None => panic!("missing LINK rule for builder {}", builder.name),
            };
            let mut link_env = Env::new();
            nested_env::merge(&mut link_env, &global_env);
            let flattened_env = nested_env::flatten(&link_env);
            let expanded =
                nested_env::expand(&link_rule.cmd, &flattened_env, IfMissing::Empty).unwrap();

            (
                NinjaRuleBuilder::default()
                    .name(&*link_rule.name)
                    .description(&*link_rule.name)
                    .command(&*expanded)
                    .build()
                    .unwrap(),
                nested_env::expand("${bindir}", &flattened_env, IfMissing::Empty).unwrap(),
            )
        };

        let bindir = PathBuf::from(bindir);
        // write compile rules & builds
        for (rule, source, out) in &app_builds {
            let rule_name = ninja_writer.write_rule_dedup(rule).unwrap();
            let mut object = bindir.clone();
            object.push(out);

            let build = NinjaBuildBuilder::default()
                .rule(&*rule_name)
                .in_single(source.as_path())
                .out(object.as_path())
                .build()
                .unwrap();
            ninja_writer.write_build(&build).unwrap();

            objects.push(object);
        }

        /* build application file name */
        let mut out_elf = bindir.clone();
        out_elf.push(&binary.name);
        let out_elf = out_elf.with_extension("elf");

        // write linking rule & build
        let link_rule_name = ninja_writer.write_rule_dedup(&ninja_link_rule).unwrap();

        // NinjaBuildBuilder expects a Vec<&Path>, but the loop above creates a Vec<PathBuf>.
        // thus, convert.
        let objects: Vec<&Path> = objects.iter().map(|pathbuf| pathbuf.as_path()).collect();

        // build ninja link target
        let ninja_link_build = NinjaBuildBuilder::default()
            .rule(&*link_rule_name)
            .in_vec(objects)
            .out(out_elf.as_path())
            .build()
            .unwrap();

        ninja_writer.write_build(&ninja_link_build).unwrap();

        // collect tasks
        let mut task_env = Env::new();
        nested_env::merge(&mut task_env, &global_env);
        task_env.insert(
            "out".into(),
            nested_env::EnvKey::Single(String::from(out_elf.to_str().unwrap())),
        );
        let flattened_task_env = nested_env::flatten(&task_env);
        let tasks = build
            .build_context
            .collect_tasks(&contexts, &flattened_task_env);

        Ok(Some(BuildInfo { tasks }))
    }

    let mut laze_env = HashMap::new();
    laze_env.insert(
        "in".to_string(),
        nested_env::EnvKey::Single("\\${in}".to_string()),
    );
    laze_env.insert(
        "out".to_string(),
        nested_env::EnvKey::Single("\\${out}".to_string()),
    );
    laze_env.insert(
        "build-dir".to_string(),
        nested_env::EnvKey::Single(build_dir.to_string_lossy().into()),
    );

    let laze_env = laze_env;

    let selected_builders = if builders.is_empty() {
        contexts.builders_vec()
    } else {
        contexts.builders_by_name(builders)
    };

    let all_apps = apps.is_empty();

    // get all "binary" modules
    let bins = contexts
        .contexts
        .iter()
        .flat_map(|ctx| ctx.modules.iter())
        .filter(|(_, module)| module.is_binary);

    // filter selected apps, if specified
    // also filter by apps in the start folder, if not in global mode
    let bins = bins.filter(move |(_, module)| {
        if !all_apps {
            if let None = apps.get(&module.name[..]) {
                return false;
            }
        }
        if !global && module.relpath.as_ref().unwrap() != start_dir {
            return false;
        }
        true
    });

    // create (builder, bin) tuples
    let builder_bin_tuples = selected_builders.iter().cartesian_product(bins);

    // actually configure builds
    let builds = builder_bin_tuples
        .filter_map(|(builder, (_, bin))| {
            match configure_build(bin, &contexts, builder, &mut ninja_writer, &laze_env).ok()? {
                Some(build_info) => Some((builder.name.clone(), bin.name.clone(), build_info)),
                _ => None,
            }
        })
        .collect::<Vec<(String, String, BuildInfo)>>();

    let num_built = builds.len();
    println!("configured {} builds.", num_built);

    Ok(builds)
}

fn determine_project_root(start: &PathBuf) -> Result<(PathBuf, PathBuf)> {
    let mut cwd = start.clone();

    loop {
        let mut tmp = cwd.clone();
        tmp.push("laze-project.yml");
        if tmp.exists() {
            return Ok((cwd, PathBuf::from("laze-project.yml")));
        }
        cwd = match cwd.parent() {
            Some(p) => PathBuf::from(p),
            None => return Err(anyhow!("cannot find laze-project.yml")),
        }
    }
}

fn ninja_run(build_dir: &Path, verbose: bool) -> Result<i32, Error> {
    let ninja_buildfile = build_dir.join("build.ninja");
    let ninja_exit = NinjaCmdBuilder::default()
        .verbose(verbose)
        .build_file(ninja_buildfile.to_str().unwrap())
        .build()
        .unwrap()
        .run()?;
    match ninja_exit.code() {
        Some(code) => {
            return Ok(code);
        }
        None => return Err(anyhow!("ninja probably killed by signal")),
    };
}

fn main() {
    let result = try_main();
    match result {
        Err(e) => {
            eprintln!("laze: error: {:#}", e);
            std::process::exit(1);
        }
        Ok(code) => std::process::exit(code),
    };
}

fn try_main() -> Result<i32> {
    let matches = App::new("laze in rust")
        .version(crate_version!())
        .author("Kaspar Schleiser <kaspar@schleiser.de>")
        .about("Build a lot, fast")
        .setting(AppSettings::InferSubcommands)
        .arg(
            Arg::with_name("chdir")
                .short("C")
                .long("chdir")
                .help("change working directory before doing anything else")
                .global(true)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("global")
                .short("g")
                .long("global")
                .help("global mode")
                .global(true)
                .required(false),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("generate build files and build")
                .arg(
                    Arg::with_name("build-dir")
                        .short("B")
                        .long("build-dir")
                        .takes_value(true)
                        .value_name("DIR")
                        .default_value("build")
                        .help("specify build dir (relative to project root)"),
                )
                .arg(
                    Arg::with_name("generate-only")
                        .short("G")
                        .long("generate-only")
                        .help("generate build files only, don't start build")
                        .required(false),
                )
                .arg(
                    Arg::with_name("builders")
                        .short("b")
                        .long("builders")
                        .help("builders to configure")
                        .required(false)
                        .takes_value(true)
                        .multiple(true)
                        .require_delimiter(true),
                )
                .arg(
                    Arg::with_name("apps")
                        .short("a")
                        .long("apps")
                        .help("apps to configure")
                        .required(false)
                        .takes_value(true)
                        .multiple(true)
                        .require_delimiter(true),
                )
                .arg(
                    Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .help("be verbose (e.g., show command lines)")
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("task")
                .about("run builder specific task")
                .usage("lazers task [FLAGS] [OPTIONS] <TASK> [ARGS]...")
                .setting(AppSettings::AllowExternalSubcommands)
                .setting(AppSettings::SubcommandRequired)
                .arg(
                    Arg::with_name("build-dir")
                        .short("B")
                        .long("build-dir")
                        .takes_value(true)
                        .value_name("DIR")
                        .default_value("build")
                        .help("specify build dir (relative to project root)"),
                )
                .arg(
                    Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .help("be verbose (e.g., show command lines)")
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("builder")
                        .short("b")
                        .long("builder")
                        .help("builder to run task for")
                        .required(false)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("app")
                        .short("a")
                        .long("app")
                        .help("application target to run task for")
                        .required(false)
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Some(dir) = matches.value_of("chdir") {
        env::set_current_dir(dir).context(format!("cannot change to directory \"{}\"", dir))?;
    }

    let cwd = env::current_dir()?;

    let (project_root, project_file) = determine_project_root(&cwd)?;
    let start_relpath = pathdiff::diff_paths(&cwd, &project_root).unwrap();

    println!(
        "laze: project root: {} relpath: {} project_file: {}",
        project_root.display(),
        start_relpath.display(),
        project_file.display()
    );

    let global = matches.is_present("global");
    env::set_current_dir(&project_root)
        .context(format!("cannot change to \"{}\"", &project_root.display()))?;

    match matches.subcommand() {
        ("build", Some(build_matches)) => {
            let verbose = build_matches.occurrences_of("verbose");
            let build_dir = Path::new(build_matches.value_of("build-dir").unwrap());

            // collect builder names from args
            let builders = build_matches
                .values_of("builders")
                .unwrap_or_default()
                .collect::<IndexSet<_>>();

            // collect app names from args
            let apps = build_matches
                .values_of("apps")
                .unwrap_or_default()
                .collect::<IndexSet<_>>();

            println!("building {:?} for {:?}", &apps, &builders);

            // arguments parsed, launch generation of ninja file(s)
            generate(
                &project_file,
                start_relpath.as_ref(),
                &build_dir,
                global,
                &builders,
                &apps,
            )?;

            if build_matches.is_present("generate-only") {
                return Ok(0);
            }

            ninja_run(build_dir, verbose > 0)?;
        }
        ("task", Some(task_matches)) => {
            let verbose = task_matches.occurrences_of("verbose");
            let build_dir = Path::new(task_matches.value_of("build-dir").unwrap());

            let builder = task_matches.value_of("builder");
            let app = task_matches.value_of("app");

            let (task, args) = match task_matches.subcommand() {
                (name, Some(matches)) => {
                    let args = matches.values_of("").map(|v| v.collect());
                    (name, args)
                }
                _ => unreachable!(),
            };

            let builders = match builder {
                Some(builder) => iter::once(builder).collect::<IndexSet<_>>(),
                None => IndexSet::new(),
            };

            let apps = match app {
                Some(app) => iter::once(app).collect::<IndexSet<_>>(),
                None => IndexSet::new(),
            };

            println!("building {:?} for {:?}", &apps, &builders);
            // arguments parsed, launch generation of ninja file(s)
            let builds = generate(
                &project_file,
                start_relpath.as_ref(),
                &build_dir,
                global,
                &builders,
                &apps,
            )?;

            let builds: Vec<&(String, String, BuildInfo)> = builds
                .iter()
                .filter(|(_, _, build_info)| build_info.tasks.contains_key(task.into()))
                .collect();

            if builds.len() > 1 {
                eprintln!("laze: multiple task targets found:");
                for (builder, bin, _build_info) in builds {
                    eprintln!("{} {}", builder, bin);
                }

                return Err(anyhow!("laze: please specify one of these."));
            }

            if builds.len() < 1 {
                return Err(anyhow!(
                    "laze: no matching target for task \"{}\" found.",
                    task
                ));
            }

            let build = builds[0];

            if ninja_run(build_dir, verbose > 0)? != 0 {
                return Err(anyhow!("laze: build error"));
            };

            println!(
                "laze: executing task {} for builder {} bin {} task",
                task, build.0, build.1,
            );

            build
                .2
                .tasks
                .get(task.into())
                .unwrap()
                .execute(project_root.as_ref(), args)?;
        }
        _ => {}
    };

    Ok(0)
}
