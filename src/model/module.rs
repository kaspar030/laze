use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use indexmap::{indexset, IndexMap, IndexSet};

use crate::download;
use crate::nested_env;
use crate::nested_env::Env;
use crate::Dependency;

#[derive(Clone, Eq, Debug)]
pub struct Module {
    pub name: String,
    pub context_name: String,

    pub selects: Vec<Dependency<String>>,
    pub imports: Vec<Dependency<String>>,
    pub conflicts: Option<Vec<String>>,
    pub notify_all: bool,

    pub blocklist: Option<Vec<String>>,
    pub allowlist: Option<Vec<String>>,

    pub sources: Vec<String>,
    pub sources_optional: Option<IndexMap<String, Vec<String>>>,

    pub build: Option<CustomBuild>,

    pub env_local: Env,
    pub env_export: Env,
    pub env_global: Env,
    pub env_early: Env,

    pub download: Option<download::Download>,
    pub context_id: Option<usize>,
    pub defined_in: Option<PathBuf>,
    pub relpath: Option<PathBuf>,
    pub srcdir: Option<PathBuf>,
    pub build_dep_files: Option<IndexSet<PathBuf>>,
    pub is_build_dep: bool,
    pub is_binary: bool,
}

impl Module {
    pub fn new(name: String, context_name: Option<String>) -> Module {
        Module {
            name,
            context_name: context_name.unwrap_or_else(|| "default".to_string()),
            selects: Vec::new(),
            imports: Vec::new(),
            conflicts: None,
            notify_all: false,
            // exports: Vec::new(),
            sources: Vec::new(),
            build: None,
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
            build_dep_files: None,
            is_build_dep: false,
            blocklist: None,
            allowlist: None,
            download: None,
        }
    }

    pub fn from(defaults: &Module, name: String, context_name: Option<String>) -> Module {
        Module {
            name,
            context_name: context_name.unwrap_or_else(|| defaults.context_name.clone()),
            selects: defaults.selects.clone(),
            imports: defaults.imports.clone(),
            conflicts: defaults.conflicts.clone(),
            notify_all: defaults.notify_all,
            // exports: Vec::new(),
            sources: defaults.sources.clone(),
            build: defaults.build.clone(),
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
            download: defaults.download.clone(),
            srcdir: defaults.srcdir.clone(),
            build_dep_files: None,
            is_build_dep: false,
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

    pub fn build_env<'a>(
        &'a self,
        global_env: &Env,
        modules: &IndexMap<&String, &'a Module>,
    ) -> (Env, Option<IndexSet<&'a Module>>) {
        /* start with the global env env */
        let mut module_env = global_env.clone();

        /* collect this module's module build deps */
        let mut build_dep_modules = None;

        /* from each (recursive) import ... */
        let deps = self.get_imports_recursive(modules, None);

        for dep in &deps {
            /* merge that dependency's exported env */
            module_env = nested_env::merge(module_env, dep.env_export.clone());

            // add all *imported (used)* dependencies to this modules "notify" env var
            // (unless it has "notify_all" set, we'll handle that later)
            if !self.notify_all {
                let notify_list = module_env
                    .entry("notify".into())
                    .or_insert_with(|| nested_env::EnvKey::List(im::vector![]));

                match notify_list {
                    nested_env::EnvKey::Single(_) => panic!("unexpected notify value"),
                    nested_env::EnvKey::List(list) => list.push_back(dep.create_module_define()),
                }
            }

            // collect all imported file build dependencies
            if dep != &self && dep.is_build_dep {
                build_dep_modules
                    .get_or_insert_with(IndexSet::new)
                    .insert(*dep);
            }
        }

        // add *all* modules to this modules "notify" env var if requested
        if self.notify_all {
            let all_modules = modules
                .iter()
                .map(|(_, dep)| dep.create_module_define())
                .collect::<im::Vector<_>>();
            module_env.insert("notify".into(), nested_env::EnvKey::List(all_modules));
        }

        /* merge the module's local env */
        module_env = nested_env::merge(module_env, self.env_local.clone());

        (module_env, build_dep_modules)
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

    // adds a Ninja target name as build dependency for this target.
    // gets env expanded on module instantiation.
    pub fn add_build_dep_file(&mut self, dep: &PathBuf) {
        if let Some(build_dep_files) = &mut self.build_dep_files {
            build_dep_files.insert(dep.clone());
        } else {
            self.build_dep_files = Some(indexset![dep.clone()]);
        }
    }

    // returns all fixed and optional sources with srcdir prepended
    // pub fn get_all_sources(&self, srcdir: PathBuf) -> Vec<PathBuf> {
    //     let mut res = self
    //         .sources
    //         .iter()
    //         .map(|source| {
    //             let mut path = srcdir.clone();
    //             path.push(source);
    //             path
    //         })
    //         .collect::<Vec<_>>();

    //     if let Some(sources_optional) = &self.sources_optional {
    //         res.extend(sources_optional.values().flatten().map(|x| {
    //             let mut path = srcdir.clone();
    //             path.push(x);
    //             path
    //         }));
    //     }

    //     res
    // }
}

impl Hash for Module {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.context_name.hash(state);
    }
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.context_name == other.context_name
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

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CustomBuild {
    pub cmd: Vec<String>,
    pub out: Option<Vec<String>>,
}
