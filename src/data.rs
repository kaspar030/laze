//! This module deals with converting laze .yml files into the format that
//! the generate module needs.
//!
//! This is intentionally separate from the main generate types in order to be a
//! bit more flexible on changes to the format.

extern crate pathdiff;
extern crate serde_yaml;

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context as _, Error, Result};
use serde::{Deserialize, Deserializer};

use treestate::{FileState, TreeState};

use super::download::Download;
use super::model::CustomBuild;
use super::nested_env::{Env, EnvKey, MergeOption};
use super::{Context, ContextBag, Dependency, Module, Rule, Task};
use crate::serde_bool_helpers::default_as_false;

mod import;
use import::Import;

pub type FileTreeState = TreeState<FileState, PathBuf>;

// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlFile {
    context: Option<Vec<YamlContext>>,
    builder: Option<Vec<YamlContext>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    module: Option<Option<Vec<YamlModule>>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    app: Option<Option<Vec<YamlModule>>>,
    import: Option<Vec<Import>>,
    subdirs: Option<Vec<String>>,
    defaults: Option<HashMap<String, YamlModule>>,
    #[serde(skip)]
    filename: Option<PathBuf>,
    #[serde(skip)]
    doc_idx: Option<usize>,
    #[serde(skip)]
    included_by: Option<usize>,
    #[serde(skip)]
    import_root: Option<ImportRoot>,
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlContext {
    name: String,
    parent: Option<String>,
    env: Option<Env>,
    select: Option<Vec<String>>,
    disable: Option<Vec<String>>,
    rule: Option<Vec<Rule>>,
    var_options: Option<im::HashMap<String, MergeOption>>,
    tasks: Option<HashMap<String, Task>>,
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
    context: Option<StringOrVecString>,
    depends: Option<Vec<StringOrMapString>>,
    selects: Option<Vec<String>>,
    uses: Option<Vec<String>>,
    disable: Option<Vec<String>>,
    #[serde(default = "default_as_false")]
    notify_all: bool,
    sources: Option<Vec<StringOrMapString>>,
    build: Option<CustomBuild>,
    env: Option<YamlModuleEnv>,
    blocklist: Option<Vec<String>>,
    allowlist: Option<Vec<String>>,
    download: Option<Download>,
    srcdir: Option<PathBuf>,
    #[serde(default = "default_as_false")]
    is_build_dep: bool,
    #[serde(skip)]
    is_binary: bool,
}

impl YamlModule {
    fn default(is_binary: bool) -> YamlModule {
        YamlModule {
            name: None,
            context: None,
            depends: None,
            selects: None,
            uses: None,
            disable: None,
            notify_all: false,
            sources: None,
            srcdir: None,
            build: None,
            env: None,
            blocklist: None,
            allowlist: None,
            download: None,
            is_build_dep: false,
            is_binary,
        }
    }

    fn get_contexts(&self) -> Vec<Option<&String>> {
        if let Some(contexts) = &self.context {
            match contexts {
                StringOrVecString::Single(single) => vec![Some(single)],
                StringOrVecString::List(list) => list.iter().map(Some).collect_vec(),
            }
        } else {
            return vec![None];
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlModuleEnv {
    local: Option<Env>,
    export: Option<Env>,
    global: Option<Env>,
}

// fn load_one<'a>(filename: &PathBuf) -> Result<YamlFile> {
//     let file = read_to_string(filename).unwrap();
//     let docs: Vec<&str> = file.split("\n---\n").collect();
//     let mut data: YamlFile = serde_yaml::from_str(&docs[0])
//         .with_context(|| format!("while parsing {}", filename.display()))?;

//     data.filename = Some(filename.clone());

//     Ok(data)
// }

fn process_removes(strings: &mut Vec<Dependency<String>>) {
    let removals = strings
        .iter()
        .filter(|x| x.get_name().starts_with('-'))
        .map(|x| x.get_name()[1..].to_string())
        .collect::<HashSet<_>>();

    strings.retain(|x| !(x.get_name().starts_with('-') || removals.contains(&x.get_name()[..])));
}

pub fn dependency_from_string(dep_name: &String) -> Dependency<String> {
    match dep_name.as_bytes()[0] {
        b'?' => Dependency::Soft(dep_name[1..].to_string()),
        _ => Dependency::Hard(dep_name.clone()),
    }
}

pub fn dependency_from_string_if(dep_name: &String, other: &String) -> Dependency<String> {
    match dep_name.as_bytes()[0] {
        b'?' => Dependency::IfThenSoft(other.clone(), dep_name[1..].to_string()),
        _ => Dependency::IfThenHard(other.clone(), dep_name.clone()),
    }
}

fn load_all<'a>(file_include: &FileInclude, index_start: usize) -> Result<Vec<YamlFile>> {
    let filename = &file_include.filename;
    let file = read_to_string(filename).with_context(|| format!("{:?}", filename))?;

    let mut result = Vec::new();
    for (n, doc) in serde_yaml::Deserializer::from_str(&file).enumerate() {
        let mut parsed =
            YamlFile::deserialize(doc).with_context(|| format!("{}", filename.display()))?;
        parsed.filename = Some(filename.clone());
        parsed.doc_idx = Some(index_start + n);
        parsed.included_by = file_include.included_by_doc_idx;
        parsed.import_root = file_include.import_root.clone();
        result.push(parsed);
    }

    Ok(result)
}

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
struct ImportRoot(PathBuf);
impl ImportRoot {
    fn path(&self) -> &Path {
        self.0.as_path()
    }
}

#[derive(Hash, Debug, PartialEq, Eq)]
struct FileInclude {
    filename: PathBuf,
    included_by_doc_idx: Option<usize>,
    import_root: Option<ImportRoot>,
}

impl FileInclude {
    fn new(
        filename: PathBuf,
        included_by_doc_idx: Option<usize>,
        import_root: Option<ImportRoot>,
    ) -> Self {
        FileInclude {
            filename,
            included_by_doc_idx,
            import_root,
        }
    }

    fn new_import(filename: PathBuf, included_by_doc_idx: Option<usize>) -> Self {
        // TODO: (opt) Cow import_root?
        let import_root = Some(ImportRoot(PathBuf::from(
            filename.parent().as_ref().unwrap(),
        )));
        FileInclude {
            filename,
            included_by_doc_idx,
            import_root,
        }
    }
}

pub fn load(filename: &Path, build_dir: &Path) -> Result<(ContextBag, FileTreeState)> {
    let mut contexts = ContextBag::new();
    let start = Instant::now();

    // yaml_datas holds all parsed yaml data
    let mut yaml_datas = Vec::new();

    // filenames contains all filenames so far included.
    // when reading files, any "subdir" will be converted to "subdir/laze.yml", then added to the
    // set.
    // using an IndexSet so files can only be added once
    let mut filenames: IndexSet<FileInclude> = IndexSet::new();
    filenames.insert(FileInclude::new(PathBuf::from(filename), None, None));

    let mut filenames_pos = 0;
    while filenames_pos < filenames.len() {
        let include = filenames.get_index(filenames_pos).unwrap();
        let filename = include.filename.clone();
        let new_index_start = yaml_datas.len();

        // load all yaml documents from filename, append to yaml_datas
        yaml_datas.append(&mut load_all(include, new_index_start)?);
        filenames_pos += 1;

        let new_index_end = yaml_datas.len();

        // iterate over newly added documents
        for i in new_index_start..new_index_end {
            let new = &yaml_datas[i];
            if let Some(subdirs) = &new.subdirs {
                let relpath = filename.parent().unwrap().to_path_buf();

                // collect subdirs, add do filenames list
                for subdir in subdirs {
                    let sub_file = Path::new(&relpath).join(subdir).join("laze.yml");
                    filenames.insert(FileInclude::new(
                        sub_file,
                        new.doc_idx,
                        new.import_root.clone(),
                    ));
                }
            }
            if let Some(imports) = &new.import {
                for import in imports {
                    // TODO: `import.handle()` does the actual git checkout (or whatever
                    // import action), so probably better handling of any errors is
                    // in order.
                    filenames.insert(FileInclude::new_import(
                        import.handle(build_dir)?,
                        new.doc_idx,
                    ));
                }
            }
        }
    }

    fn convert_context(
        context: &YamlContext,
        contexts: &mut ContextBag,
        is_builder: bool,
        filename: &PathBuf,
        import_root: &Option<ImportRoot>,
    ) -> Result<(), Error> {
        let context_name = &context.name;
        let context_parent = match &context.parent {
            Some(x) => x.clone(),
            None => "default".to_string(),
        };
        // println!(
        //     "{} {} parent {}",
        //     match is_builder {
        //         true => "builder",
        //         false => "context",
        //     },
        //     context_name,
        //     context_parent,
        // );
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
            .with_context(|| format!("{:?}: adding context \"{}\"", &filename, &context_name))?;
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
        context_.var_options = context.var_options.clone();
        // populate "early env"
        let relpath = {
            let relpath = filename.parent().unwrap().to_str().unwrap();
            if relpath.is_empty() {
                ".".to_string()
            } else {
                relpath.to_string()
            }
        };

        context_
            .env_early
            .insert("relpath".into(), EnvKey::Single(relpath));
        if let Some(import_root) = import_root {
            context_.env_early.insert(
                "root".into(),
                EnvKey::Single(import_root.path().to_str().unwrap().into()),
            );
        } else {
            context_
                .env_early
                .insert("root".into(), EnvKey::Single(".".into()));
        }

        if let Some(tasks) = &context.tasks {
            let flattened_early_env = crate::nested_env::flatten(&context_.env_early);
            context_.tasks = Some(
                tasks
                    .iter()
                    .map(|(name, task)| (name.clone(), task.with_env(&flattened_early_env)))
                    .collect(),
            )
        }

        context_.apply_early_env();

        context_.defined_in = Some(filename.clone());

        context_.disable = context.disable.clone();

        // collect context level "select:"
        if let Some(select) = &context.select {
            context_.select = Some(
                select
                    .iter()
                    .map(dependency_from_string)
                    .collect::<Vec<_>>(),
            );
        }

        Ok(())
    }

    fn init_module(
        name: &Option<String>,
        context: Option<&String>,
        is_binary: bool,
        filename: &PathBuf,
        import_root: &Option<ImportRoot>,
        defaults: Option<&Module>,
    ) -> Module {
        let relpath = filename.parent().unwrap().to_str().unwrap().to_string();
        let name = match name {
            Some(name) => name.clone(),
            None => {
                if let Some(import_root) = import_root {
                    filename
                        .parent()
                        .unwrap()
                        .strip_prefix(import_root.path())
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                } else {
                    relpath.clone()
                }
            }
        };

        let mut module = match defaults {
            Some(defaults) => Module::from(defaults, name, context.cloned()),
            None => Module::new(name, context.cloned()),
        };

        module.is_binary = is_binary;
        module.defined_in = Some(filename.clone());
        module.relpath = Some(PathBuf::from(&relpath));

        module
    }

    fn convert_module(
        module: &YamlModule,
        context: Option<&String>,
        is_binary: bool,
        filename: &PathBuf,
        import_root: &Option<ImportRoot>,
        defaults: Option<&Module>,
        build_dir: &Path,
    ) -> Result<Module, Error> {
        let relpath = filename.parent().unwrap().to_str().unwrap().to_string();

        let mut m = init_module(
            &module.name,
            context,
            is_binary,
            filename,
            import_root,
            defaults,
        );

        // convert module dependencies
        // "selects" means "module will be part of the build"
        // "uses" means "if module is part of the build, transitively import its exported env vars"
        // "depends" means both select and use a module
        // a build configuration fails if a selected or depended on module is not
        // available.
        // "disable" is only valid for binaries ("apps"), and will make any module
        // with the specified name unavailable in the binary's build context
        //
        if let Some(selects) = &module.selects {
            // println!("selects:");
            for dep_name in selects {
                // println!("- {}", dep_name);
                m.selects.push(dependency_from_string(dep_name));
            }
        }
        if let Some(uses) = &module.uses {
            // println!("uses:");
            for dep_name in uses {
                // println!("- {}", dep_name);
                m.imports.push(dependency_from_string(dep_name));
            }
        }
        if let Some(depends) = &module.depends {
            // println!("depends:");
            for dep_spec in depends {
                match dep_spec {
                    StringOrMapString::String(dep_name) => {
                        // println!("- {}", dep_name);
                        m.selects.push(dependency_from_string(dep_name));
                        m.imports.push(dependency_from_string(dep_name));
                    }
                    StringOrMapString::Map(dep_map) => {
                        for (k, v) in dep_map {
                            // println!("- {}:", k);
                            for dep_name in v {
                                // println!("  - {}", dep_name);
                                m.selects.push(dependency_from_string_if(dep_name, k));
                                m.imports.push(dependency_from_string_if(dep_name, k));
                            }
                        }
                    }
                }
            }
        }

        if let Some(disable) = &module.disable {
            if m.disable.is_none() {
                m.disable = Some(Vec::new());
            }
            for dep_name in disable {
                m.disable.as_mut().unwrap().push(dep_name.clone());
            }
        }

        if module.notify_all {
            m.notify_all = true;
        }

        // if a module name starts with "-", remove it from the list, also the
        // same name without "-".
        // this allows adding e.g., a dependency in "default: ...", but removing
        // it later. add/remove/add won't work, though.
        process_removes(&mut m.selects);
        process_removes(&mut m.imports);

        // copy over environment
        if let Some(env) = &module.env {
            if let Some(local) = &env.local {
                m.env_local = super::nested_env::merge(m.env_local, local.clone());
            }
            if let Some(export) = &env.export {
                m.env_export = super::nested_env::merge(m.env_export, export.clone());
            }
            if let Some(global) = &env.global {
                m.env_global = super::nested_env::merge(m.env_global, global.clone());
            }
        }

        if let Some(sources) = &module.sources {
            let mut sources_optional = IndexMap::new();
            for source in sources {
                match source {
                    StringOrMapString::String(source) => m.sources.push(source.clone()),
                    StringOrMapString::Map(source) => {
                        // collect optional sources into sources_optional
                        for (k, v) in source {
                            let list = sources_optional.entry(k).or_insert(Vec::new());
                            for entry in v {
                                list.push(entry.clone());
                            }
                        }
                    }
                }
            }

            // if there are optional sources, merge them into the module's
            // optional sources map
            if !sources_optional.is_empty() {
                if m.sources_optional.is_none() {
                    m.sources_optional = Some(IndexMap::new());
                }
                let m_sources_optional = m.sources_optional.as_mut().unwrap();
                for (k, v) in sources_optional {
                    let list = m_sources_optional.entry(k.clone()).or_insert(Vec::new());
                    for entry in v {
                        list.push(entry.clone());
                    }
                }
            }
        }

        if let Some(defaults_blocklist) = &mut m.blocklist {
            if let Some(module_blocklist) = &module.blocklist {
                defaults_blocklist.append(&mut (module_blocklist.clone()));
            }
        } else {
            m.blocklist = module.blocklist.clone();
        }

        if let Some(defaults_allowlist) = &mut m.allowlist {
            if let Some(module_allowlist) = &module.allowlist {
                defaults_allowlist.append(&mut (module_allowlist.clone()));
            }
        } else {
            m.allowlist = module.allowlist.clone();
        }

        m.download = module.download.clone();
        let srcdir = if let Some(download) = &m.download {
            let srcdir = download.srcdir(build_dir, &m);
            let tagfile = download.tagfile(&srcdir);

            m.add_build_dep_file(&tagfile);

            // if a module has downloaded files, always consider it to be a
            // build dependency, as all dependees / users might include e.g.,
            // downloaded headers.
            m.is_build_dep = true;

            srcdir
        } else {
            PathBuf::from(relpath.clone())
        };

        m.build = module.build.clone();

        if m.download.is_none() {
            // if a module has downloaded source, it is already a build dependency
            // for dependees / users. Otherwise, do what the user thinks.
            m.is_build_dep = module.is_build_dep;
        }

        m.srcdir = module
            .srcdir
            .as_ref()
            .map_or(Some(srcdir), |s| Some(PathBuf::from(s)));

        // populate "early env"
        m.env_early
            .insert("relpath".into(), EnvKey::Single(relpath));
        if let Some(import_root) = import_root {
            m.env_early.insert(
                "root".into(),
                EnvKey::Single(import_root.path().to_str().unwrap().into()),
            );
        } else {
            m.env_early
                .insert("root".into(), EnvKey::Single(".".into()));
        }
        m.env_early.insert(
            "srcdir".into(),
            EnvKey::Single(m.srcdir.as_ref().unwrap().to_str().unwrap().into()),
        );

        m.env_local = crate::nested_env::merge(m.env_local, m.env_early.clone());
        m.apply_early_env();

        Ok(m)
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
                        &mut contexts,
                        *is_builder,
                        data.filename.as_ref().unwrap(),
                        &data.import_root,
                    )?;
                }
            }
        }
    }

    // after this, there's a default context, context relationships and envs have been set up.
    // modules can now be processed.
    contexts.finalize()?;

    // for context in &contexts.contexts {
    //     if let Some(env) = &context.env {
    //         println!("context {} env:", context.name);
    //         dbg!(env);
    //     }
    // }
    let mut subdir_module_defaults_map = HashMap::new();
    let mut subdir_app_defaults_map = HashMap::new();

    fn get_defaults(
        data: &YamlFile,
        defaults_map: &HashMap<usize, Module>,
        key: &str,
        is_binary: bool,
        build_dir: &Path,
    ) -> Option<Module> {
        // this function determines the module or app defaults for a given YamlFile

        // determine inherited "defaults: module: ..."
        let subdir_defaults: Option<&Module> = if let Some(included_by) = &data.included_by {
            defaults_map.get(included_by)
        } else {
            None
        };

        // determine "defaults: module: ..." from yaml document
        let mut module_defaults = if let Some(defaults) = &data.defaults {
            if let Some(module_defaults) = defaults.get(key) {
                let context = &module_defaults
                    .context
                    .as_ref()
                    .and_then(|context| match context {
                        StringOrVecString::List(_) => {
                            panic!("module defaults with context _list_")
                        }
                        StringOrVecString::Single(context) => Some(context),
                    });

                Some(
                    convert_module(
                        module_defaults,
                        *context,
                        is_binary,
                        data.filename.as_ref().unwrap(),
                        &data.import_root,
                        subdir_defaults,
                        build_dir,
                    )
                    .unwrap(),
                )
            } else {
                None
            }
        } else {
            None
        };
        if module_defaults.is_none() {
            if let Some(subdir_defaults) = subdir_defaults {
                module_defaults = Some(subdir_defaults.clone());
            }
        }
        module_defaults
    }

    for data in &yaml_datas {
        let module_defaults = get_defaults(
            data,
            &subdir_module_defaults_map,
            "module",
            false,
            build_dir,
        );
        let app_defaults = get_defaults(data, &subdir_app_defaults_map, "app", true, build_dir);

        if data.subdirs.is_some() {
            if let Some(module_defaults) = &module_defaults {
                subdir_module_defaults_map.insert(data.doc_idx.unwrap(), module_defaults.clone());
            }
            if let Some(app_defaults) = &app_defaults {
                subdir_app_defaults_map.insert(data.doc_idx.unwrap(), app_defaults.clone());
            }
        }

        for (list, is_binary) in [(&data.module, false), (&data.app, true)].iter() {
            if let Some(module_list) = list {
                if let Some(module_list) = module_list {
                    for module in module_list {
                        for context in module.get_contexts() {
                            contexts.add_module(convert_module(
                                module,
                                context,
                                *is_binary,
                                data.filename.as_ref().unwrap(),
                                &data.import_root,
                                if *is_binary {
                                    app_defaults.as_ref()
                                } else {
                                    module_defaults.as_ref()
                                },
                                build_dir,
                            )?)?;
                        }
                    }
                } else if *is_binary {
                    // if an app list is empty, add a default entry.
                    // this allows a convenient file only containing "app:"
                    let module = YamlModule::default(*is_binary);
                    for context in module.get_contexts() {
                        contexts.add_module(convert_module(
                            &module,
                            context,
                            *is_binary,
                            data.filename.as_ref().unwrap(),
                            &data.import_root,
                            app_defaults.as_ref(),
                            build_dir,
                        )?)?;
                    }
                }
            }
        }
    }

    println!(
        "laze: reading {} files took {:?}",
        filenames.len(),
        start.elapsed(),
    );

    let start = Instant::now();
    let treestate = FileTreeState::new(filenames.iter().map(|include| &include.filename));
    println!(
        "laze: stat'ing {} files took {:?}",
        filenames.len(),
        start.elapsed(),
    );

    Ok((contexts, treestate))
}
