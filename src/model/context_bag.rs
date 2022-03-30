use std::collections::HashMap;
use std::{iter::Filter, slice::Iter};

use anyhow::Error;
use indexmap::IndexSet;

use super::{BlockAllow, Context, Module};
use crate::nested_env;

pub struct ContextBag {
    pub contexts: Vec<Context>,
    pub context_map: HashMap<String, usize>,
}

pub enum IsAncestor {
    No,
    Yes(usize, usize),
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

    pub fn finalize(&mut self) -> Result<(), Error> {
        /* ensure there's a "default" context */
        if let None = self.get_by_name(&"default".to_string()) {
            self.add_context(Context::new("default".to_string(), None))
                .unwrap();
        }

        /* set the "parent" index of each context that sets a "parent_name" */
        for context in &mut self.contexts {
            if let Some(parent_name) = &context.parent_name {
                let parent = self.context_map.get(&parent_name.clone()).ok_or_else(|| {
                    anyhow!(format!(
                        "{:?}: context \"{}\" has unknown parent \"{}\"",
                        context.defined_in.as_ref().unwrap().as_os_str(),
                        &context.name,
                        &parent_name
                    ))
                })?;
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
            let context_env = &context.env;
            let parent_env = &self.contexts[context.parent_index.unwrap()].env;

            if let Some(parent_env) = parent_env {
                let env;
                if let Some(context_env) = context_env {
                    env = nested_env::merge(parent_env.clone(), context_env.clone());
                } else {
                    env = parent_env.clone();
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

        Ok(())
    }

    pub fn add_context_or_builder(
        &mut self,
        mut context: Context,
        is_builder: bool,
    ) -> Result<&mut Context, Error> {
        if let Some(context_id) = self.context_map.get(&context.name) {
            let context = self.context_by_id(*context_id);
            return Err(anyhow!(
                "context name already defined in {:?}",
                context.defined_in.as_ref().unwrap()
            ));
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

    pub fn add_module(&mut self, mut module: Module) -> Result<(), Error> {
        let context_id = self.context_map.get(&module.context_name).ok_or_else(|| {
            anyhow!(format!(
                "{:?}: module \"{}\": undefined context \"{}\"",
                module.defined_in.as_ref().unwrap().as_os_str(),
                &module.name,
                &module.context_name
            ))
        })?;

        let context = &mut self.contexts[*context_id];
        module.context_id = Some(*context_id);
        match context.modules.entry(module.name.clone()) {
            indexmap::map::Entry::Occupied(other_module) => {
                return Err(anyhow!(
                    "{:?}: module \"{}\", context \"{}\": module name already used in {:?}",
                    module.defined_in.as_ref().unwrap().as_os_str(),
                    &module.name,
                    &module.context_name,
                    other_module.get().defined_in.as_ref().unwrap().as_os_str(),
                ))
            }
            indexmap::map::Entry::Vacant(entry) => {
                entry.insert(module);
            }
        }
        Ok(())
    }

    pub fn builders(&self) -> Filter<Iter<Context>, fn(&&Context) -> bool> {
        self.contexts.iter().filter(|&x| x.is_builder)
    }

    pub fn builders_vec(&self) -> Vec<&Context> {
        self.builders().collect()
    }

    pub fn builders_by_name(&self, names: &IndexSet<String>) -> Result<Vec<&Context>, Error> {
        let mut res = Vec::new();
        for name in names {
            match self.get_by_name(name) {
                Some(context) => {
                    if context.is_builder {
                        res.push(context);
                    } else {
                        return Err(anyhow!(format!(
                            "context {} is not a build context",
                            &context.name
                        )));
                    }
                }
                None => return Err(anyhow!(format!("unknown builder {}", &name))),
            }
        }
        Ok(res)
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
