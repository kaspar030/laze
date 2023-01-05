use anyhow::{Context as _, Error, Result};
use indexmap::{IndexMap, IndexSet};

use crate::model::{Context, ContextBag, Dependency, Module};
use crate::nested_env::{self, Env};

pub struct Build<'a> {
    bag: &'a ContextBag,
    binary: Module,
    builder: &'a Context,
    pub build_context: Context,
    //modules: IndexMap<&'a String, &'a Module>,
}

struct Resolver<'a> {
    build: &'a Build<'a>,
    module_set: IndexMap<&'a String, &'a Module>,
    if_then_deps: IndexMap<String, Vec<Dependency<String>>>,
    disabled_modules: IndexSet<String>,
    provided_by: IndexMap<&'a String, Vec<&'a Module>>,
}

pub struct ResolverResult<'a> {
    pub modules: IndexMap<&'a String, &'a Module>,
    pub providers: IndexMap<&'a String, Vec<&'a Module>>,
}

#[derive(Debug)]
struct ResolverState {
    module_set_prev_len: usize,
    if_then_deps_prev_len: usize,
    disabled_modules_prev_len: usize,
    provided_prev_len: usize,
}

impl<'a> Resolver<'a> {
    fn new(build: &'a Build<'a>, disabled_modules: IndexSet<String>) -> Resolver<'a> {
        Self {
            build,
            module_set: IndexMap::new(),
            if_then_deps: IndexMap::new(),
            provided_by: IndexMap::new(),
            disabled_modules,
        }
    }

    fn state(&self) -> ResolverState {
        ResolverState {
            module_set_prev_len: self.module_set.len(),
            if_then_deps_prev_len: self.if_then_deps.len(),
            disabled_modules_prev_len: self.disabled_modules.len(),
            provided_prev_len: self.provided_by.len(),
        }
    }

    fn reset(&mut self, state: ResolverState) {
        self.module_set.truncate(state.module_set_prev_len);
        self.if_then_deps.truncate(state.if_then_deps_prev_len);
        self.disabled_modules
            .truncate(state.disabled_modules_prev_len);
        self.provided_by.truncate(state.provided_prev_len)
    }

    fn result(self) -> ResolverResult<'a> {
        ResolverResult {
            modules: self.module_set,
            providers: self.provided_by,
        }
    }

    fn resolve_module_name_deep(&mut self, module_name: &String) -> Result<(), Error> {
        let (_context, module) = match self
            .build
            .build_context
            .resolve_module(module_name, self.build.bag)
        {
            Some(x) => x,
            None => return Err(anyhow!("module \"{}\" not found", module_name)),
        };

        self.resolve_module_deep(module)
    }

    fn resolve_module_deep(&mut self, module: &'a Module) -> Result<(), Error> {
        let state = self.state();
        if self.module_set.contains_key(&module.name) {
            return Ok(());
        }

        if self.disabled_modules.contains(&module.name) {
            return Err(anyhow!("\"{}\" is disabled/conflicted", module.name));
        }

        if let Some(conflicts) = &module.conflicts {
            for conflicted in conflicts {
                if self.module_set.contains_key(conflicted) {
                    self.reset(state);
                    return Err(anyhow!("\"{}\" conflicts \"{}\"", module.name, conflicted));
                }
                if self.provided_by.contains_key(conflicted) {
                    self.reset(state);
                    return Err(anyhow!(
                        "\"{}\" conflicts already provided \"{}\"",
                        module.name,
                        conflicted
                    ));
                }
            }
        }

        // handle "provides" of this module.
        // also, bail out if any of the provided modules has been conflicted
        // before, which implicitly conflicts this module.
        if let Some(provides) = &module.provides {
            for provided in provides {
                if self.disabled_modules.contains(provided) {
                    self.reset(state);
                    bail!("provides disabled/conflicted module \"{}\"", provided);
                }
            }
        }

        // register this module's conflicts
        if let Some(conflicts) = &module.conflicts {
            self.disabled_modules.extend(conflicts.iter().cloned());
        }

        // all provided modules get added to the "provided_by" map, so later
        // dependees of one of those get informed.
        if let Some(provides) = &module.provides {
            for name in provides {
                self.add_provided_by(name, module);
            }
        }

        // if self.provided_set.contains(dep_name) {
        //     optional = true;
        // }

        self.module_set.insert(&module.name, module);

        // late if_then_deps are dependencies that are induced by if_then_deps of
        // other modules.
        // e.g., A -> if (B) then C
        // if_then_deps contains "A: B -> C"
        // Now if B gets resolved, C is now also a dependency.
        let mut late_if_then_deps = Vec::new();
        if let Some(deps) = self.if_then_deps.get(&module.name) {
            late_if_then_deps.extend(deps.iter().cloned());
        }

        for dep in module.selects.iter().chain(late_if_then_deps.iter()) {
            let (dep_name, mut optional) = match dep {
                Dependency::Hard(name) => (name, false),
                Dependency::Soft(name) => (name, true),
                Dependency::IfThenHard(other, name) => {
                    if self.module_set.contains_key(other) {
                        (name, false)
                    } else {
                        self.if_then_deps
                            .entry(other.clone())
                            .or_insert_with(Vec::new)
                            .push(Dependency::Hard(name.clone()));
                        continue;
                    }
                }
                Dependency::IfThenSoft(other, name) => {
                    if self.module_set.contains_key(other) {
                        (name, true)
                    } else {
                        self.if_then_deps
                            .entry(other.clone())
                            .or_insert_with(Vec::new)
                            .push(Dependency::Soft(name.clone()));

                        continue;
                    }
                }
            };

            // TODO: (consistency): this should be handled *after* modules
            // which match the exact name
            if let Some(provides) = &self.build.build_context.provides {
                if let Some(providing_modules) = provides.get(dep_name) {
                    if self.resolve_module_list(providing_modules, dep_name) > 0 {
                        optional = true;
                        // resolve_module_deep should handle this:
                        //self.provided_set.insert(dep_name.clone());
                        if self.disabled_modules.contains(dep_name) {
                            // one provider conflicted the dependency name,
                            // we'll need to skip the possible exact matching
                            // module.
                            continue;
                        }
                    }
                }
            }

            if let Err(err) = self
                .resolve_module_name_deep(dep_name)
                .with_context(|| format!("\"{}\" cannot resolve \"{}\"", module.name, dep_name))
            {
                if optional {
                    continue;
                } else {
                    self.reset(state);
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    fn add_provided_by(&mut self, name: &'a String, module: &'a Module) {
        self.provided_by
            .entry(name)
            .or_insert_with(Vec::new)
            .push(module);
    }

    fn resolve_module_list(
        &mut self,
        providing_modules: &IndexSet<String>,
        provided_name: &String,
    ) -> usize {
        let mut count = 0usize;
        for module_name in providing_modules {
            if self.module_set.contains_key(module_name) {
                // this module is already selected. up the count and try next
                count += 1;
                continue;
            }

            if self.disabled_modules.contains(provided_name) {
                // this "provides" is conflicted, so no other provider can
                // be chosen. if we already have one hit, we're done.
                // otherwise, we continue to see if a possible later candidate
                // is already in the modules set, in which case the dependency
                // is met.
                if count > 0 {
                    break;
                } else {
                    continue;
                }
            }

            if self.resolve_module_name_deep(module_name).is_ok() {
                count += 1;
            }
        }
        count
    }
}

impl<'a: 'b, 'b> Build<'b> {
    pub fn new(
        binary: &'a Module,
        builder: &'a Context,
        contexts: &'a ContextBag,
        cli_selects: Option<&Vec<Dependency<String>>>,
    ) -> Build<'b> {
        let mut build_context = Context::new_build_context(builder.name.clone(), builder);

        if let Some(parent) = build_context.get_parent(contexts) {
            build_context.provides = parent.provides.clone();
        }

        // TODO: opt: see if Cow improves performance
        let mut binary = binary.clone();

        // collect all selects for this build.
        // the order (and thus precedence) is:
        // 1. cli
        // 2. app
        // 3. context
        binary.selects = cli_selects
            .iter()
            .flat_map(|x| x.iter())
            .cloned()
            .chain(binary.selects.drain(..))
            .chain(build_context.collect_selected_modules(contexts).drain(..))
            .collect();

        let mut build = Build {
            bag: contexts,
            binary,
            builder,
            build_context,
        };

        // fixup name to "$builder_name:$binary_name"
        build.build_context.name.push(':');
        build.build_context.name.push_str(&build.binary.name);

        // collect environment from builder
        let mut build_env;
        if let Some(builder_env) = &builder.env {
            build_env = builder_env.clone();
        } else {
            build_env = Env::new();
        }

        // insert "builder" variable
        build_env.insert(
            "builder".to_string(),
            nested_env::EnvKey::Single(builder.name.clone()),
        );
        // add "app" variable
        build_env.insert(
            "app".to_string(),
            nested_env::EnvKey::Single(build.binary.name.clone()),
        );

        build.build_context.env = Some(build_env);

        build
    }

    pub fn resolve_selects(
        &self,
        disabled_modules: IndexSet<String>,
    ) -> Result<ResolverResult, Error> {
        let mut resolver = Resolver::new(self, disabled_modules);

        resolver
            .resolve_module_deep(&self.binary)
            .with_context(|| {
                format!(
                    "binary \"{}\" for builder \"{}\"",
                    self.binary.name, self.builder.name
                )
            })?;

        Ok(resolver.result())
    }
}
