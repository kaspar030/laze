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
use std::time::Instant;

use anyhow::{Context as _, Error, Result};
use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;
use serde::{Deserialize, Deserializer};

use treestate::{FileState, TreeState};

use super::download::Download;
use super::model::CustomBuild;
use super::nested_env::{Env, EnvKey, MergeOption};
use super::{Context, ContextBag, Dependency, Module, Rule, Task};
use crate::serde_bool_helpers::default_as_false;
use crate::utils::StringOrMapVecString;

mod import;
use import::Import;

pub type FileTreeState = TreeState<FileState, std::path::PathBuf>;

// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

fn deserialize_version_checked<'de, D>(deserializer: D) -> Result<Option<Version>, D::Error>
where
    //    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    use serde::de;

    let version: Option<String> = Deserialize::deserialize(deserializer)?;
    if let Some(version) = &version {
        if let Ok(version) = Version::parse(version) {
            let my_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
            if version > my_version {
                return Err(de::Error::custom(format!(
                    "laze_required_version: got \"{version}\", expected any version <={my_version}"
                )));
            }
            Ok(Some(version))
        } else {
            return Err(de::Error::custom(format!(
                "error parsing \"{version}\" as semver version string"
            )));
        }
    } else {
        Ok(None)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlFile {
    contexts: Option<Vec<YamlContext>>,
    builders: Option<Vec<YamlContext>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    modules: Option<Option<Vec<YamlModule>>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    apps: Option<Option<Vec<YamlModule>>>,
    imports: Option<Vec<Import>>,
    subdirs: Option<Vec<String>>,
    defaults: Option<HashMap<String, YamlModule>>,
    #[serde(default, deserialize_with = "deserialize_version_checked")]
    laze_required_version: Option<Version>,
    #[serde(skip)]
    filename: Option<Utf8PathBuf>,
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
    selects: Option<Vec<String>>,
    disables: Option<Vec<String>>,
    rules: Option<Vec<Rule>>,
    var_options: Option<im::HashMap<String, MergeOption>>,
    tasks: Option<HashMap<String, Task>>,
    #[serde(skip)]
    _is_builder: bool,
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
    depends: Option<Vec<StringOrMapVecString>>,
    selects: Option<Vec<String>>,
    uses: Option<Vec<String>>,
    provides: Option<Vec<String>>,
    provides_unique: Option<Vec<String>>,
    #[serde(alias = "disables")]
    conflicts: Option<Vec<String>>,
    #[serde(default = "default_as_false")]
    notify_all: bool,
    sources: Option<Vec<StringOrMapVecString>>,
    build: Option<CustomBuild>,
    env: Option<YamlModuleEnv>,
    blocklist: Option<Vec<String>>,
    allowlist: Option<Vec<String>>,
    download: Option<Download>,
    srcdir: Option<Utf8PathBuf>,
    #[serde(default = "default_as_false")]
    is_build_dep: bool,
    #[serde(default = "default_as_false")]
    is_global_build_dep: bool,
    #[serde(skip)]
    _is_binary: bool,
}

impl YamlModule {
    fn default(is_binary: bool) -> YamlModule {
        YamlModule {
            name: None,
            context: None,
            depends: None,
            selects: None,
            uses: None,
            provides: None,
            provides_unique: None,
            conflicts: None,
            notify_all: false,
            sources: None,
            srcdir: None,
            build: None,
            env: None,
            blocklist: None,
            allowlist: None,
            download: None,
            is_build_dep: false,
            is_global_build_dep: false,
            _is_binary: is_binary,
        }
    }

    fn get_contexts(&self) -> Vec<Option<&String>> {
        if let Some(contexts) = &self.context {
            match contexts {
                StringOrVecString::Single(single) => vec![Some(single)],
                StringOrVecString::List(list) => list.iter().map(Some).collect_vec(),
            }
        } else {
            vec![None]
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct YamlModuleEnv {
    local: Option<Env>,
    export: Option<Env>,
    global: Option<Env>,
}

// fn load_one<'a>(filename: &Utf8PathBuf) -> Result<YamlFile> {
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

pub fn dependency_from_string_if(dep_name: &String, other: &str) -> Dependency<String> {
    match dep_name.as_bytes()[0] {
        b'?' => Dependency::IfThenSoft(other.to_string(), dep_name[1..].to_string()),
        _ => Dependency::IfThenHard(other.to_string(), dep_name.clone()),
    }
}

fn load_all(file_include: &FileInclude, index_start: usize) -> Result<Vec<YamlFile>> {
    let filename = &file_include.filename;
    let file = read_to_string(filename).with_context(|| format!("{:?}", filename))?;

    let mut result = Vec::new();
    for (n, doc) in serde_yaml::Deserializer::from_str(&file).enumerate() {
        let mut parsed = YamlFile::deserialize(doc).with_context(|| filename.clone())?;
        parsed.filename = Some(filename.clone());
        parsed.doc_idx = Some(index_start + n);
        parsed.included_by = file_include.included_by_doc_idx;
        parsed.import_root = file_include.import_root.clone();
        result.push(parsed);
    }

    Ok(result)
}

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
struct ImportRoot(Utf8PathBuf);
impl ImportRoot {
    fn path(&self) -> &Utf8Path {
        self.0.as_path()
    }
}

#[derive(Hash, Debug, PartialEq, Eq)]
struct FileInclude {
    filename: Utf8PathBuf,
    included_by_doc_idx: Option<usize>,
    import_root: Option<ImportRoot>,
}

impl FileInclude {
    fn new(
        filename: Utf8PathBuf,
        included_by_doc_idx: Option<usize>,
        import_root: Option<ImportRoot>,
    ) -> Self {
        FileInclude {
            filename,
            included_by_doc_idx,
            import_root,
        }
    }

    fn new_import(filename: Utf8PathBuf, included_by_doc_idx: Option<usize>) -> Self {
        // TODO: (opt) Cow import_root?
        let import_root = Some(ImportRoot(Utf8PathBuf::from(
            filename.parent().as_ref().unwrap(),
        )));
        FileInclude {
            filename,
            included_by_doc_idx,
            import_root,
        }
    }
}

pub fn load(filename: &Utf8Path, build_dir: &Utf8Path) -> Result<(ContextBag, FileTreeState)> {
    let mut contexts = ContextBag::new();
    let start = Instant::now();

    // yaml_datas holds all parsed yaml data
    let mut yaml_datas = Vec::new();

    // filenames contains all filenames so far included.
    // when reading files, any "subdir" will be converted to "subdir/laze.yml", then added to the
    // set.
    // using an IndexSet so files can only be added once
    let mut filenames: IndexSet<FileInclude> = IndexSet::new();
    filenames.insert(FileInclude::new(Utf8PathBuf::from(filename), None, None));

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
        for new in yaml_datas[new_index_start..new_index_end].iter() {
            if let Some(subdirs) = &new.subdirs {
                let relpath = filename.parent().unwrap().to_path_buf();

                // collect subdirs, add do filenames list
                for subdir in subdirs {
                    let sub_file = Utf8Path::new(&relpath).join(subdir).join("laze.yml");
                    filenames.insert(FileInclude::new(
                        sub_file,
                        new.doc_idx,
                        new.import_root.clone(),
                    ));
                }
            }
            if let Some(imports) = &new.imports {
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
        filename: &Utf8PathBuf,
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
        let context_ = contexts
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
        if let Some(rules) = &context.rules {
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
            let relpath = filename.parent().unwrap().as_str();
            if relpath.is_empty() {
                ".".to_string()
            } else {
                relpath.to_string()
            }
        };

        context_
            .env_early
            .insert("relpath".into(), EnvKey::Single(relpath));

        context_.env_early.insert(
            "root".into(),
            EnvKey::Single(match import_root {
                Some(import_root) => import_root.path().to_string(),
                None => ".".into(),
            }),
        );

        if let Some(tasks) = &context.tasks {
            let flattened_early_env = context_.env_early.flatten()?;
            context_.tasks = Some(
                tasks
                    .iter()
                    .map(|(name, task)| {
                        let task = task.with_env(&flattened_early_env);
                        match task {
                            Ok(task) => Ok((name.clone(), task)),
                            Err(e) => Err(e),
                        }
                        .with_context(|| format!("task \"{}\"", name.clone()))
                    })
                    .collect::<Result<_, _>>()
                    .with_context(|| format!("{:?} context \"{}\"", &filename, context.name))?,
            )
        }

        context_.apply_early_env()?;

        context_.defined_in = Some(filename.clone());

        context_.disable = context.disables.clone();

        // collect context level "select:"
        if let Some(select) = &context.selects {
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
        filename: &Utf8Path,
        import_root: &Option<ImportRoot>,
        defaults: Option<&Module>,
    ) -> Module {
        let relpath = filename.parent().unwrap();

        let name = match name {
            Some(name) => name.clone(),
            None => if let Some(import_root) = import_root {
                filename
                    .parent()
                    .unwrap()
                    .strip_prefix(import_root.path())
                    .unwrap()
            } else {
                &relpath
            }
            .to_string(),
        };

        let mut module = match defaults {
            Some(defaults) => Module::from(defaults, name, context.cloned()),
            None => Module::new(name, context.cloned()),
        };

        module.is_binary = is_binary;
        module.defined_in = Some(filename.to_path_buf());
        module.relpath = Some(if relpath.eq("") {
            Utf8PathBuf::from(".")
        } else {
            Utf8PathBuf::from(&relpath)
        });

        module
    }

    fn convert_module(
        module: &YamlModule,
        context: Option<&String>,
        is_binary: bool,
        filename: &Utf8Path,
        import_root: &Option<ImportRoot>,
        defaults: Option<&Module>,
        build_dir: &Utf8Path,
    ) -> Result<Module, Error> {
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
        // "conflicts" will make any module with the specified name unavailable
        // in the binary's build context, if the module specifying a conflict
        // is part of the dependency tree
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
                    StringOrMapVecString::String(dep_name) => {
                        // println!("- {}", dep_name);
                        m.selects.push(dependency_from_string(dep_name));
                        m.imports.push(dependency_from_string(dep_name));
                    }
                    StringOrMapVecString::Map(dep_map) => {
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

        if let Some(conflicts) = &module.conflicts {
            add_conflicts(&mut m, conflicts);
        }

        if let Some(provides) = &module.provides {
            if let Some(default_provides) = m.provides {
                let mut provides = provides.clone();
                let mut res = default_provides;
                res.append(&mut provides);
                m.provides = Some(res);
            } else {
                m.provides = Some(provides.clone());
            }
        }

        if let Some(provides_unique) = &module.provides_unique {
            // a "uniquely provided module" requires to be the only provider
            // for that module. think `provides_unique: [ libc ]`.
            // practically, it means adding to both "provides" and "conflicts"
            add_conflicts(&mut m, provides_unique);
            if m.provides.is_none() {
                m.provides = Some(Vec::new());
            }
            m.provides
                .as_mut()
                .unwrap()
                .append(&mut provides_unique.clone());
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
                m.env_local.merge(&local);
            }
            if let Some(export) = &env.export {
                m.env_export.merge(&export);
            }
            if let Some(global) = &env.global {
                m.env_global.merge(&global);
            }
        }

        if let Some(sources) = &module.sources {
            let mut sources_optional = IndexMap::new();
            for source in sources {
                match source {
                    StringOrMapVecString::String(source) => m.sources.push(source.clone()),
                    StringOrMapVecString::Map(source) => {
                        // collect optional sources into sources_optional
                        for (k, v) in source {
                            let list: &mut Vec<String> = sources_optional.entry(k).or_default();
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
                    let list = m_sources_optional.entry(k.clone()).or_default();
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

        let relpath = m.relpath.as_ref().unwrap().clone();

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
            if relpath != "." {
                relpath.clone()
            } else {
                "".into()
            }
        };

        m.build = module.build.clone();
        m.is_global_build_dep = module.is_global_build_dep;

        if m.download.is_none() {
            // if a module has downloaded source, it is already a build dependency
            // for dependees / users. Otherwise, do what the user thinks.
            m.is_build_dep = module.is_build_dep;
        }

        m.srcdir = module
            .srcdir
            .as_ref()
            .map_or(Some(srcdir), |s| Some(Utf8PathBuf::from(s)));

        // populate "early env"
        m.env_early
            .insert("relpath".into(), EnvKey::Single(relpath.to_string()));

        m.env_early.insert(
            "root".into(),
            EnvKey::Single(match import_root {
                Some(import_root) => import_root.path().to_string(),
                None => ".".into(),
            }),
        );
        m.env_early.insert(
            "srcdir".into(),
            EnvKey::Single(m.srcdir.as_ref().unwrap().as_path().to_string()),
        );

        m.env_local.merge(&m.env_early);
        m.apply_early_env()?;

        if is_binary {
            m.env_global
                .insert("appdir".into(), EnvKey::Single(relpath.to_string()));
        }

        Ok(m)
    }

    // collect and convert contexts
    // this needs to be done before collecting modules, as that requires
    // contexts to be finalized.
    for data in &yaml_datas {
        for (list, is_builder) in [(&data.contexts, false), (&data.builders, true)].iter() {
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
        build_dir: &Utf8Path,
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

        for (list, is_binary) in [(&data.modules, false), (&data.apps, true)].iter() {
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

    contexts.merge_provides();

    println!(
        "laze: reading {} files took {:?}",
        filenames.len(),
        start.elapsed(),
    );

    let start = Instant::now();

    // convert Utf8PathBufs to PathBufs
    // TODO: make treestate support camino Utf8PathBuf
    let filenames = filenames
        .drain(..)
        .map(|include| include.filename.into_std_path_buf())
        .collect_vec();

    let treestate = FileTreeState::new(filenames.iter());
    println!(
        "laze: stat'ing {} files took {:?}",
        filenames.len(),
        start.elapsed(),
    );

    Ok((contexts, treestate))
}

fn add_conflicts(m: &mut Module, conflicts: &Vec<String>) {
    if m.conflicts.is_none() {
        m.conflicts = Some(Vec::new());
    }
    for dep_name in conflicts {
        m.conflicts.as_mut().unwrap().push(dep_name.clone());
    }
}
