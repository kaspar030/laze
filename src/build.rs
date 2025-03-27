use anyhow::{Context as _, Error, Result};
use im_rc::{HashMap, HashSet, Vector};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::model::{Context, ContextBag, Dependency, Module};
use crate::nested_env::{self, Env};

pub struct Build<'a> {
    bag: &'a ContextBag,
    binary: Module,
    builder: &'a Context,
    pub build_context: Context,
    //modules: IndexMap<&'a String, &'a Module>,
}

struct Resolver<'a, const VERBOSE: bool> {
    build: &'a Build<'a>,
    state_stack: Vec<ResolverState<'a>>,
    state: ResolverState<'a>,
}

pub struct ResolverResult<'a> {
    pub modules: IndexMap<&'a String, &'a Module>,
    pub providers: IndexMap<&'a String, Vec<&'a Module>>,
}

#[derive(Debug, Clone, Default)]
struct ResolverState<'a> {
    module_set: HashSet<&'a String>,
    module_list: Vector<(&'a String, &'a Module)>,
    if_then_deps: HashMap<String, Vector<Dependency<String>>>,
    disabled_modules: HashMap<String, HashSet<&'a String>>,
    provided_by: HashMap<&'a String, Vector<&'a Module>>,
}

impl<'a, const VERBOSE: bool> Resolver<'a, VERBOSE> {
    fn new(build: &'a Build<'a>, mut disabled_modules: IndexSet<String>) -> Self {
        let mut disabled_modules_map: HashMap<String, HashSet<&String>> = HashMap::new();
        for module_name in disabled_modules.drain(..) {
            disabled_modules_map.entry(module_name).or_default();
        }
        Self {
            build,
            state_stack: Vec::new(),
            state: ResolverState {
                disabled_modules: disabled_modules_map,
                ..Default::default()
            },
        }
    }

    fn state_push(&mut self) {
        let new_state = self.state.clone();
        let old_state = core::mem::replace(&mut self.state, new_state);
        self.state_stack.push(old_state);
    }

    fn state_pop(&mut self) {
        self.state = self
            .state_stack
            .pop()
            .expect("should not pop below last stack");
    }

    fn state_indent(&self) -> String {
        let mut res = String::new();
        for _ in 0..self.state_stack.len() {
            res.push_str("  ");
        }
        res
    }

    fn result(self) -> ResolverResult<'a> {
        let modules = self
            .state
            .module_list
            .iter()
            .map(|(x, y)| (*x, *y))
            .collect();
        let providers = self
            .state
            .provided_by
            .iter()
            .map(|(x, y)| {
                let vec = y.iter().cloned().collect();
                (*x, vec)
            })
            .collect();

        ResolverResult { modules, providers }
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

    fn trace<F>(&self, f: F)
    where
        F: FnOnce() -> String,
    {
        if VERBOSE {
            println!("{}{}", self.state_indent(), f());
        }
    }

    fn resolve_module_deep(&mut self, module: &'a Module) -> Result<(), Error> {
        self.trace(|| format!("resolving {}", module.name));
        if self.state.module_set.contains(&module.name) {
            self.trace(|| format!("resolving {}: already selected", module.name));
            return Ok(());
        }

        if let Some(disabled_by) = self.state.disabled_modules.get(&module.name) {
            let disabled_by = disabled_by.iter().join(", ");
            self.trace(|| format!("resolving {}: disabled by {disabled_by}", module.name));
            return Err(anyhow!(
                "\"{}\" is disabled/conflicted by {disabled_by}",
                module.name
            ));
        }

        if let Some(conflicts) = &module.conflicts {
            for conflicted in conflicts {
                if self.state.module_set.contains(conflicted) {
                    self.trace(|| format!("resolving {}: conflicts {conflicted}", module.name));
                    return Err(anyhow!("\"{}\" conflicts \"{}\"", module.name, conflicted));
                }

                if let Some(others) = self.state.provided_by.get(conflicted) {
                    let others_wrapped: Vec<String> =
                        others.iter().map(|m| format!("\"{}\"", m.name)).collect();
                    self.trace(|| {
                        format!(
                            "resolving {}: conflicts already provided {conflicted} (provided by: {})",
                            module.name, others_wrapped.join(", ")
                        )
                    });
                    return Err(anyhow!(
                        "\"{}\" conflicts already provided \"{}\" (by {})",
                        module.name,
                        conflicted,
                        others_wrapped.join(", ")
                    ));
                }
            }
        }

        // handle "provides" of this module.
        // also, bail out if any of the provided modules has been conflicted
        // before, which implicitly conflicts this module.
        if let Some(provides) = &module.provides {
            for provided in provides {
                if let Some(disabled_by) = self.state.disabled_modules.get(provided) {
                    let disabled_by = disabled_by.iter().join(", ");
                    let msg = format!(
                        "provides `{provided}` which is disabled/conflicted by {disabled_by}"
                    );
                    self.trace(|| format!("resolving {}: {msg}", module.name));
                    return Err(anyhow!(msg));
                }
            }
        }

        // new state needed here
        self.state_push();

        // register this module's conflicts
        if let Some(conflicts) = &module.conflicts {
            for conflicted in conflicts {
                self.state
                    .disabled_modules
                    .entry(conflicted.clone())
                    .or_default()
                    .insert(&module.name);
            }
        }

        // all provided modules get added to the "provided_by" map, so later
        // dependees of one of those get informed.
        if let Some(provides) = &module.provides {
            for name in provides {
                self.add_provided_by(name, module);
            }
        }

        self.state.module_set.insert(&module.name);
        self.state.module_list.push_back((&module.name, module));

        // late if_then_deps are dependencies that are induced by if_then_deps of
        // other modules.
        // e.g., A -> if (B) then C
        // if_then_deps contains "A: B -> C"
        // Now if B gets resolved, C is now also a dependency.
        let mut late_if_then_deps = Vec::new();
        if let Some(deps) = self.state.if_then_deps.get(&module.name) {
            late_if_then_deps.extend(deps.iter().cloned());
        }

        for dep in module.selects.iter().chain(late_if_then_deps.iter()) {
            let (dep_name, optional) = match dep {
                Dependency::Hard(name) => (name, false),
                Dependency::Soft(name) => (name, true),
                Dependency::IfThenHard(other, name) => {
                    if self.state.module_set.contains(other) {
                        (name, false)
                    } else {
                        self.state
                            .if_then_deps
                            .entry(other.clone())
                            .or_default()
                            .push_back(Dependency::Hard(name.clone()));
                        continue;
                    }
                }
                Dependency::IfThenSoft(other, name) => {
                    if self.state.module_set.contains(other) {
                        (name, true)
                    } else {
                        self.state
                            .if_then_deps
                            .entry(other.clone())
                            .or_default()
                            .push_back(Dependency::Soft(name.clone()));

                        continue;
                    }
                }
            };

            // TODO: (consistency): this should be handled *after* modules
            // which match the exact name
            let mut was_provided = false;
            if let Some(provided) = &self.build.build_context.provided {
                if let Some(providing_modules) = provided.get(dep_name) {
                    if self.resolve_module_list(providing_modules, dep_name) > 0 {
                        self.trace(|| format!("got at least one provider for `{dep_name}`"));
                        was_provided = true;
                        if self.state.disabled_modules.contains_key(dep_name) {
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
                self.trace(|| format!("resolving {dep_name}: failed (optional={optional}, was_provided={was_provided})"));

                if optional || was_provided {
                    continue;
                } else {
                    self.state_pop();
                    return Err(err);
                }
            }
        }

        self.state_stack.pop();

        Ok(())
    }

    fn add_provided_by(&mut self, name: &'a String, module: &'a Module) {
        self.state
            .provided_by
            .entry(name)
            .or_default()
            .push_back(module);
    }

    fn resolve_module_list(
        &mut self,
        providing_modules: &IndexSet<String>,
        provided_name: &String,
    ) -> usize {
        self.trace(|| format!("resolving provided name {provided_name}"));
        let mut count = 0usize;
        for module_name in providing_modules {
            if self.state.module_set.contains(module_name) {
                // this module is already selected. up the count and try next
                self.trace(|| format!("  `{module_name}` already selected"));
                count += 1;
                continue;
            }

            if self.state.disabled_modules.contains_key(provided_name) {
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
            build_context.provided.clone_from(&parent.provided);
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
            .chain(std::iter::once(Dependency::Hard(builder.module_name())))
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
        verbose: bool,
    ) -> Result<ResolverResult, Error> {
        if verbose {
            let mut resolver = Resolver::<true>::new(self, disabled_modules);
            resolver
                .resolve_module_deep(&self.binary)
                .with_context(|| {
                    format!(
                        "binary \"{}\" for builder \"{}\"",
                        self.binary.name, self.builder.name
                    )
                })?;

            Ok(resolver.result())
        } else {
            let mut resolver = Resolver::<false>::new(self, disabled_modules);
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
}
