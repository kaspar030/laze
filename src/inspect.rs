//! Inspect Laze project files, without any build output

use std::io::Write;

use anyhow::Result;
use camino::Utf8PathBuf;
use ptree::{write_tree, TreeBuilder};

use crate::{data::load, Context, ContextBag};

pub(crate) struct BuildInspector {
    contexts: ContextBag,
}

impl BuildInspector {
    pub(crate) fn from_project(project_file: Utf8PathBuf, build_dir: Utf8PathBuf) -> Result<Self> {
        let (contexts, _, _) = load(&project_file, &build_dir)?;
        Ok(Self { contexts })
    }
    pub(crate) fn inspect_builders(&self) -> Vec<&Context> {
        self.contexts.builders_vec()
    }

    fn add_tree_element(&self, context: &Context, tree: &mut TreeBuilder) {
        self.contexts
            .contexts
            .iter()
            .filter(|c| Some(context) == c.get_parent(&self.contexts))
            .for_each(|c| {
                tree.begin_child(c.name.to_string());
                self.add_tree_element(c, tree);
                tree.end_child();
            })
    }

    pub(crate) fn write_tree<W: Write>(&self, w: W) -> Result<()> {
        let default = self.contexts.get_by_name(&"default".to_string()).unwrap();
        let mut tree_builder = TreeBuilder::new("default".to_string()); // The first node is always called default
        self.add_tree_element(default, &mut tree_builder);
        let tree = tree_builder.build();
        write_tree(&tree, w).map_err(|e| e.into())
    }
}
