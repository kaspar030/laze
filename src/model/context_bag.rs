use std::collections::HashMap;
use std::{iter::Filter, slice::Iter};

use camino::Utf8PathBuf;
use indexmap::IndexSet;
use thiserror::Error;

use super::{BlockAllow, Context, Module};

#[derive(Default)]
pub struct ContextBag {
    pub contexts: Vec<Context>,
    pub context_map: HashMap<String, usize>,
    contexts_topo_sorted: Option<Vec<(usize, usize)>>,
}

pub enum IsAncestor {
    No,
    Yes(usize, usize),
}

#[derive(Error, Debug, Clone)]
pub enum ContextBagError {
    #[error("{defined_in:?}: context \"{name}\" has unknown parent \"{parent_name}\"")]
    UnknownParent {
        defined_in: Utf8PathBuf,
        name: String,
        parent_name: String,
    },
    #[error(r#"context name already defined in "{defined_in}""#)]
    DuplicateContext { defined_in: Utf8PathBuf },

    #[error(r#"{defined_in:?}: module "{name}": undefined context "{context_name}""#)]
    UndefinedModuleContext {
        defined_in: Utf8PathBuf,
        name: String,
        context_name: String,
    },

    #[error(r#"{defined_in:?}: module "{name}", context "{context_name}": module name already used in {conflict_in:?}"#)]
    DuplicateModule {
        defined_in: Utf8PathBuf,
        name: String,
        context_name: String,
        conflict_in: Utf8PathBuf,
    },

    #[error("context {name} is not a build context")]
    NotABuildContext { name: String },
    #[error("unknown builder {name}")]
    UnknownBuilder { name: String },
}

impl ContextBag {
    pub fn new() -> ContextBag {
        Self::default()
    }

    pub fn get_by_name(&self, name: &String) -> Option<&Context> {
        self.context_map.get(name).map(|id| &self.contexts[*id])
    }

    pub fn finalize(&mut self) -> Result<(), ContextBagError> {
        // ensure there's a "default" context
        if self.get_by_name(&"default".to_string()).is_none() {
            self.add_context(Context::new_default()).unwrap();
        }

        // set the "parent" index of each context that sets a "parent_name"
        for context in &mut self.contexts {
            if let Some(parent_name) = &context.parent_name {
                let parent = self.context_map.get(&parent_name.clone()).ok_or_else(|| {
                    ContextBagError::UnknownParent {
                        defined_in: context.defined_in.clone().unwrap(),
                        name: context.name.clone(),
                        parent_name: parent_name.into(),
                    }
                })?;
                context.parent_index = Some(*parent);
            }
        }

        // merge environments of parent context, recursively. to do that,
        // we need to ensure that we process the contexts in an order so that each context is
        // processed after all its parents have been processed.
        // This can be done by topologically sorting them by the total numbers of parents.
        // Also, collect var_options for each builder.
        //

        // 1. sort by number of parents (ascending)
        let mut sorted_by_numparents: Vec<_> = self
            .contexts
            .iter()
            .enumerate()
            .map(|(n, context)| (n, context.count_parents(self)))
            .collect();

        sorted_by_numparents.sort_by(|a, b| a.1.cmp(&b.1));

        // 2. merge ordered by number of parents (ascending)
        for (n, m) in &sorted_by_numparents {
            let (n, m) = (*n, *m);
            if m == 0 {
                continue;
            }

            let context = &self.contexts[n];
            let context_env = &context.env;
            let parent_env = &self.contexts[context.parent_index.unwrap()].env;

            if let Some(parent_env) = parent_env {
                let mut env = parent_env.clone();
                if let Some(context_env) = context_env {
                    env.merge(context_env);
                }
                let context = &mut self.contexts[n];
                context.env = Some(env);
            }
        }

        for (n, m) in &sorted_by_numparents {
            let (n, m) = (*n, *m);
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
            let combined_var_opts = parent_var_ops.as_ref().map(|parent_var_ops| {
                context.var_options.as_ref().map_or_else(
                    || parent_var_ops.clone(),
                    |context_var_opts| {
                        parent_var_ops
                            .into_iter()
                            .chain(context_var_opts)
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    },
                )
            });

            let context = &mut self.contexts[n];

            if context.var_options.is_none() {
                context.var_options = combined_var_opts;
            }
        }

        self.contexts_topo_sorted = Some(sorted_by_numparents);

        Ok(())
    }

    pub fn add_context_or_builder(
        &mut self,
        mut context: Context,
        is_builder: bool,
    ) -> Result<&mut Context, ContextBagError> {
        if let Some(context_id) = self.context_map.get(&context.name) {
            let context = self.context_by_id(*context_id);
            return Err(ContextBagError::DuplicateContext {
                defined_in: context.defined_in.clone().unwrap(),
            }
            .into());
        }

        let last = self.contexts.len();

        context.is_builder = is_builder;
        context.index = Some(last);

        self.context_map.insert(context.name.clone(), last);
        self.contexts.push(context);

        Ok(&mut self.contexts[last])
    }

    pub fn add_context(&mut self, context: Context) -> Result<&mut Context, ContextBagError> {
        self.add_context_or_builder(context, false)
    }

    pub fn add_module(&mut self, mut module: Module) -> Result<(), ContextBagError> {
        let context_id = self.context_map.get(&module.context_name).ok_or_else(|| {
            ContextBagError::UndefinedModuleContext {
                defined_in: module.defined_in.clone().unwrap(),
                name: module.name.clone(),
                context_name: module.context_name.clone(),
            }
        })?;
        let context = &mut self.contexts[*context_id];
        module.context_id = Some(*context_id);
        match context.modules.entry(module.name.clone()) {
            indexmap::map::Entry::Occupied(other_module) => {
                return Err(ContextBagError::DuplicateModule {
                    defined_in: module.defined_in.clone().unwrap(),
                    name: module.name,
                    context_name: module.context_name,
                    conflict_in: other_module.get().defined_in.clone().unwrap(),
                })
            }
            indexmap::map::Entry::Vacant(entry) => {
                if let Some(provides) = &module.provides {
                    let context_provides = context.provided.get_or_insert_with(im::HashMap::new);
                    for provided in provides {
                        context_provides
                            .entry(provided.clone())
                            .or_default()
                            .insert(module.name.clone());
                    }
                }
                entry.insert(module);
            }
        }
        Ok(())
    }

    pub fn builders(&self) -> Filter<Iter<'_, Context>, fn(&&Context) -> bool> {
        self.contexts.iter().filter(|&x| x.is_builder)
    }

    #[allow(clippy::type_complexity)]
    pub fn modules<'a>(
        &'a self,
    ) -> std::iter::FlatMap<
        Iter<'a, Context>,
        indexmap::map::Iter<'a, String, Module>,
        impl FnMut(&'a Context) -> indexmap::map::Iter<'a, String, Module>,
    > {
        self.contexts.iter().flat_map(|c| c.modules.iter())
    }

    pub fn builders_vec(&self) -> Vec<&Context> {
        self.builders().collect()
    }

    pub fn builders_by_name(
        &self,
        names: &IndexSet<String>,
    ) -> Result<Vec<&Context>, ContextBagError> {
        let mut res = Vec::new();
        for name in names {
            match self.get_by_name(name) {
                Some(context) => {
                    if context.is_builder {
                        res.push(context);
                    } else {
                        return Err(ContextBagError::NotABuildContext {
                            name: context.name.clone(),
                        });
                    }
                }
                None => return Err(ContextBagError::UnknownBuilder { name: name.into() }),
            }
        }
        Ok(res)
    }

    pub fn builder_distances(&self, name: impl AsRef<str>) -> Vec<(usize, &Context)> {
        let mut distances: Vec<_> = self
            .builders()
            .map(|b| (edit_distance::edit_distance(name.as_ref(), &b.name), b))
            .collect();

        distances.sort_by(|a, b| a.0.cmp(&b.0));
        distances
    }

    pub fn closest_builder_within(
        &self,
        name: impl AsRef<str>,
        max_distance: usize,
    ) -> Option<&Context> {
        if let Some((distance, context)) = self.builder_distances(name).first() {
            if *distance <= max_distance {
                return Some(context);
            }
        }
        None
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

    pub fn is_allowed(
        &self,
        context: &Context,
        blocklist: &Option<Vec<String>>,
        allowlist: &Option<Vec<String>>,
    ) -> BlockAllow {
        let allowlist_entry = allowlist.as_ref().map_or(IsAncestor::No, |list| {
            self.is_ancestor_in_list(context, list)
        });
        let blocklist_entry = blocklist.as_ref().map_or(IsAncestor::No, |list| {
            self.is_ancestor_in_list(context, list)
        });

        if allowlist.is_some() {
            if blocklist.is_some() {
                if let IsAncestor::Yes(allow_index, allow_depth) = allowlist_entry {
                    if let IsAncestor::Yes(block_index, block_depth) = blocklist_entry {
                        if allow_depth > block_depth {
                            return BlockAllow::block(block_index, block_depth);
                        }
                    }
                    return BlockAllow::allow(allow_index, allow_depth);
                } else if let IsAncestor::Yes(block_index, block_depth) = blocklist_entry {
                    return BlockAllow::block(block_index, block_depth);
                }
            } else if let IsAncestor::No = allowlist_entry {
                return BlockAllow::Blocked;
            }
        } else if blocklist.is_some() {
            if let IsAncestor::Yes(block_index, block_depth) = blocklist_entry {
                return BlockAllow::block(block_index, block_depth);
            }
        }

        BlockAllow::Allowed
    }

    pub fn merge_provides(&mut self) {
        // if "other" has parent "default",
        // and if there's a module "foo" providing "bar" in context "default",
        // and "foobar" providing "bar" in context "other",
        // then "other"'s "provides" map will be "bar: [ foobar, foo]"
        // and "default"'s "provides" map will be "bar: [ foo ]"
        for (n, m) in self.contexts_topo_sorted.as_ref().unwrap() {
            let (n, m) = (*n, *m);
            if m == 0 {
                // no parents, nothing to join
                continue;
            }
            let context = &self.contexts[n];
            let provides = &context.provided;
            let parent_provides = &self.contexts[context.parent_index.unwrap()].provided;
            let combined_provides = {
                if let Some(provides) = provides {
                    if let Some(parent_provides) = parent_provides {
                        Some(provides.clone().union_with_key(
                            parent_provides.clone(),
                            |_provided, context_set, parent_set| {
                                context_set
                                    .union(&parent_set)
                                    .cloned()
                                    .collect::<IndexSet<_>>()
                            },
                        ))
                    } else {
                        Some(provides.clone())
                    }
                } else {
                    parent_provides.as_ref().cloned()
                }
            };

            // it is possible after merging that a parent registered "bar provided by foo",
            // but a child context has its own "foo" which does not provide bar.
            // thus, filter out those cases.
            // TODO: (opt) this looks inefficient
            let context_modules = &self.contexts[n].modules;
            let combined_provides = combined_provides.map(|mut provides| {
                provides
                    .iter_mut()
                    .map(|(provided, provider)| {
                        provider.retain(|providing_name| {
                            if let Some(module) = context_modules.get(providing_name) {
                                if let Some(provides) = &module.provides {
                                    if provides.contains(provided) {
                                        return true;
                                    }
                                }
                                return false;
                            }
                            true
                        });
                        (provided.clone(), provider.clone())
                    })
                    .collect()
            });

            let context = &mut self.contexts[n];
            context.provided = combined_provides;
        }
    }

    // pub fn get_by_name_mut(&mut self, name: &String) -> Option<&mut Context> {
    //     let id = self.context_map.get(name);
    //     match id {
    //         Some(id) => Some(&mut self.contexts[*id]),
    //         None => None,
    //     }
    // }

    // pub fn print(&self) {
    //     for context in &self.contexts {
    //         let parent_name = match context.parent_index {
    //             Some(index) => self.contexts[index].name.clone(),
    //             None => "none".to_string(),
    //         };

    //         println!("context: {} parent: {}", context.name, parent_name);
    //         for (_, module) in &context.modules {
    //             println!("        {}", module.name);
    //         }
    //     }
    // }
}
