extern crate pathdiff;
extern crate serde_yaml;

use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::nested_env::Env;
use super::{Context, ContextBag, Module, Rule};

use anyhow::{Context as _, Result};

#[derive(Debug, Serialize, Deserialize)]
struct YamlFile {
    context: Option<Vec<YamlContext>>,
    builder: Option<Vec<YamlContext>>,
    module: Option<Vec<YamlModule>>,
    app: Option<Vec<YamlModule>>,
    import: Option<Vec<String>>,
    subdirs: Option<Vec<String>>,
    #[serde(skip)]
    filename: Option<PathBuf>,
    #[serde(skip)]
    included_from: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlContext {
    name: String,
    parent: Option<String>,
    env: Option<Env>,
    rule: Option<Vec<Rule>>,
    #[serde(skip)]
    is_builder: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrMapString {
    String(String),
    Map(HashMap<String, Vec<String>>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrVecString {
    Single(String),
    List(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlModule {
    name: Option<String>,
    context: Option<String>,
    depends: Option<Vec<StringOrMapString>>,
    selects: Option<Vec<String>>,
    uses: Option<Vec<String>>,
    sources: Option<Vec<StringOrMapString>>,
    env: Option<YamlModuleEnv>,
    #[serde(skip)]
    is_binary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlModuleEnv {
    local: Option<Env>,
    export: Option<Env>,
    global: Option<Env>,
}

fn load_one<'a>(filename: &PathBuf) -> Result<YamlFile> {
    let file = read_to_string(filename).unwrap();
    let docs: Vec<&str> = file.split("\n---\n").collect();
    let mut data: YamlFile = serde_yaml::from_str(&docs[0])
        .with_context(|| format!("while parsing {}", filename.display()))?;

    data.filename = Some(filename.clone());

    Ok(data)
}

pub fn load<'a>(filename: &Path, contexts: &'a mut ContextBag) -> Result<&'a ContextBag> {
    let start = Instant::now();

    // yaml_datas holds all parsed yaml data
    let mut yaml_datas = Vec::new();

    // filenames contains all filenames so far included.
    // when reading files, any "subdir" will be converted to "subdir/laze.yml", then added to the
    // set.
    // using a set so checking for already included is fast.
    let mut filenames = IndexSet::new();
    filenames.insert(PathBuf::from(filename));

    let mut filenames_pos = 0;
    while filenames_pos < filenames.len() {
        let filename = filenames.get_index(filenames_pos).unwrap();
        yaml_datas.push(load_one(filename)?);
        filenames_pos += 1;

        let last = &yaml_datas[yaml_datas.len() - 1];
        if let Some(subdirs) = &last.subdirs {
            let relpath = filename.parent().unwrap().to_path_buf();
            for subdir in subdirs {
                let mut sub_relpath = relpath.clone();
                sub_relpath.push(subdir);
                let mut sub_file = sub_relpath.clone();
                sub_file.push("laze.yml");
                filenames.insert(sub_file);
            }
        }
    }

    fn convert_context(
        context: &YamlContext,
        contexts: &mut ContextBag,
        is_builder: bool,
        filename: &PathBuf,
    ) {
        let context_name = &context.name;
        let context_parent = match &context.parent {
            Some(x) => x.clone(),
            None => "default".to_string(),
        };
        println!(
            "{} {} parent {}",
            match is_builder {
                true => "builder",
                false => "context",
            },
            context_name,
            context_parent,
        );
        let mut context_ = contexts
            .add_context_or_builder(
                Context::new(
                    context_name.clone(),
                    match context_name.as_str() {
                        "default" => None,
                        _ => Some(context_parent),
                    },
                ),
                is_builder,
            )
            .unwrap();
        context_.env = context.env.clone();
        if let Some(rules) = &context.rule {
            context_.rules = Some(IndexMap::new());
            for rule in rules {
                let mut rule = rule.clone();
                rule.context = Some(context_name.clone());
                context_
                    .rules
                    .as_mut()
                    .unwrap()
                    .insert(rule.name.clone(), rule);
            }
        }
        context_.defined_in = Some(filename.clone());
    }

    fn convert_module(module: &YamlModule, is_binary: bool, filename: &PathBuf) -> Module {
        let module_name = match &module.name {
            Some(name) => name.clone(),
            None => filename.parent().unwrap().to_str().unwrap().to_string(),
        };
        let mut m = Module::new(module_name.clone(), module.context.clone());
        println!(
            "{} {}:{}",
            match is_binary {
                true => "binary".to_string(),
                false => "module".to_string(),
            },
            module.context.as_ref().unwrap_or(&"none".to_string()),
            module_name
        );
        // convert module dependencies
        // "selects" means "module will be part of the build"
        // "uses" means "if module is part of the build, transitively import its exported env vars"
        // "depends" means both select and use a module
        // a build configuration fails if a selected or depended on module is not
        // available.
        if let Some(selects) = &module.selects {
            println!("selects:");
            for dep_name in selects {
                println!("- {}", dep_name);
                m.selects.push(dep_name.clone());
            }
        }
        if let Some(uses) = &module.uses {
            println!("uses:");
            for dep_name in uses {
                println!("- {}", dep_name);
                m.imports.push(dep_name.clone());
            }
        }
        if let Some(depends) = &module.depends {
            println!("depends:");
            for dep_spec in depends {
                if let StringOrMapString::String(dep_name) = dep_spec {
                    println!("- {}", dep_name);
                    m.selects.push(dep_name.clone());

                    // when "depends" are specified, they can be prefixed with "?"
                    // to make them optional (depend on when available, ignore if not).
                    // as all imports are optional, remove trailing "?" if present.
                    let import_name = if dep_name.starts_with("?") {
                        dep_name[1..].to_string()
                    } else {
                        dep_name.clone()
                    };
                    m.imports.push(import_name);
                } else {
                    println!("warning: optional dependency map not implemented (ignored)");
                }
            }
        }

        // copy over environment
        if let Some(env) = &module.env {
            if let Some(local) = &env.local {
                super::nested_env::merge(&mut m.env_local, local);
            }
            if let Some(export) = &env.export {
                super::nested_env::merge(&mut m.env_export, export);
            }
            if let Some(global) = &env.global {
                super::nested_env::merge(&mut m.env_global, global);
            }
        }

        if let Some(sources) = &module.sources {
            let mut sources_ = Vec::new();
            for source in sources {
                match source {
                    StringOrMapString::String(source) => sources_.push(source.clone()),
                    StringOrMapString::Map(source) => continue,
                }
            }
            m.sources = sources_;
        }

        m.is_binary = is_binary;
        m.defined_in = Some(filename.clone());
        m
    }

    // collect and convert contexts
    // this needs to be done before collecting modules, as that requires
    // contexts to be finalized.
    for data in &yaml_datas {
        for (list, is_builder) in [(&data.context, false), (&data.builder, true)].iter() {
            if let Some(context_list) = list {
                for context in context_list {
                    convert_context(
                        context,
                        contexts,
                        *is_builder,
                        &data.filename.as_ref().unwrap(),
                    );
                }
            }
        }
    }

    /* after this, there's a default context, context relationships and envs have been set up.
    modules can now be processed. */
    contexts.finalize();

    // for context in &contexts.contexts {
    //     if let Some(env) = &context.env {
    //         println!("context {} env:", context.name);
    //         dbg!(env);
    //     }
    // }

    for data in &yaml_datas {
        for (list, is_binary) in [(&data.module, false), (&data.app, true)].iter() {
            if let Some(module_list) = list {
                for module in module_list {
                    contexts
                        .add_module(convert_module(
                            &module,
                            *is_binary,
                            &data.filename.as_ref().unwrap(),
                        ))
                        .unwrap();
                }
            }
        }
    }

    println!(
        "laze: reading {} files took {:?}",
        filenames.len(),
        start.elapsed(),
    );

    Ok(contexts)
}
