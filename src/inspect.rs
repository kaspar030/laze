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

    fn add_mermaid_relation(&self, context: &Context, mermaid: &mut String) {
        self.contexts
            .contexts
            .iter()
            .filter(|c| Some(context) == c.get_parent(&self.contexts))
            .for_each(|c| {
                let parent = c.get_parent(&self.contexts).unwrap();
                mermaid.push_str(&format!(
                    r#"  {}["{}"] --> {}"#,
                    c.name.replace('-', "_"),
                    c.name,
                    parent.name.replace('-', "_")
                ));
                mermaid.push('\n');
                self.add_mermaid_relation(c, mermaid);
            })
    }

    pub(crate) fn gen_mermaid(&self) -> Result<String> {
        let mut mermaid = String::from("graph RL\n");
        let default = self.contexts.get_by_name(&"default".to_string()).unwrap();
        self.add_mermaid_relation(default, &mut mermaid);
        Ok(mermaid)
    }

    pub(crate) fn render_svg(&self) -> Result<String> {
        let mermaid = self.gen_mermaid()?;
        let mut theme = mermaid_rs_renderer::Theme::modern();
        theme.font_family = "'DejaVu Sans', 'trebuchet ms', verdana, arial, sans-serif".into();
        let opts = mermaid_rs_renderer::RenderOptions {
            theme,
            layout: mermaid_rs_renderer::LayoutConfig {
                rank_spacing: 100.0,
                node_spacing: 100.0,
                ..Default::default()
            },
        };
        mermaid_rs_renderer::render_with_options(&mermaid, opts)
    }

    pub(crate) fn gen_png(&self) -> Result<image::RgbaImage> {
        let svg = self.render_svg()?;
        let mut opt = usvg::Options {
            font_family: "'DejaVu Sans', 'trebuchet ms', verdana, arial, sans-serif".into(),
            ..Default::default()
        };
        opt.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_str(&svg, &opt)?;
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
            .ok_or_else(|| anyhow::anyhow!("Failed to allocate pixmap"))?;
        let mut pixmap_mut = pixmap.as_mut();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::default(),
            &mut pixmap_mut,
        );
        Ok(image::RgbaImage::from_raw(size.width(), size.height(), pixmap.take()).unwrap())
    }
}
