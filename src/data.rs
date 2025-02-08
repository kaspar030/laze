//! This module deals with converting laze .yml files into the format that
//! the generate module needs.
//!
//! This is intentionally separate from the main generate types in order to be a
//! bit more flexible on changes to the format.

extern crate pathdiff;
extern crate serde_yaml;

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Error, Result};
use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;
use serde::{Deserialize, Deserializer};

use treestate::{FileState, TreeState};

use super::download::Download;
use super::model::CustomBuild;
use super::nested_env::{Env, EnvKey, MergeOption};
use super::{Context, ContextBag, Dependency, Module, Rule, Task};
use crate::serde_bool_helpers::{default_as_false, default_as_true};
use crate::utils::{StringOrMapString, StringOrMapVecString};

mod import;
use import::ImportEntry;

pub type FileTreeState = TreeState<FileState, std::path::PathBuf>;

pub struct LoadStats {
    pub files: usize,
    pub parsing_time: Duration,
    pub stat_time: Duration,
}

// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

fn default_none<T>() -> Option<T> {
    None
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
                    "laze_required_version >= {version}, but this is laze {my_version}"
                )));
            }
            Ok(Some(version))
        } else {
            Err(de::Error::custom(format!(
                "error parsing \"{version}\" as semver version string"
            )))
        }
    } else {
        Ok(None)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct YamlFile {
    contexts: Option<Vec<YamlContext>>,
    builders: Option<Vec<YamlContext>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    modules: Option<Option<Vec<YamlModule>>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    apps: Option<Option<Vec<YamlModule>>>,
    imports: Option<Vec<ImportEntry>>,
    includes: Option<Vec<String>>,
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
    #[serde(rename = "meta")]
    _meta: Option<Value>,
}

fn check_module_name<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<String>::deserialize(deserializer)?;

    if let Some(v) = v.as_ref() {
        if v.starts_with("context::") {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"a string not starting with \"context::\"",
            ));
        }
    }

    Ok(v)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct YamlContext {
    name: String,
    parent: Option<String>,
    help: Option<String>,
    env: Option<Env>,
    selects: Option<Vec<String>>,
    disables: Option<Vec<String>>,
    rules: Option<Vec<YamlRule>>,
    var_options: Option<im::HashMap<String, MergeOption>>,
    tasks: Option<HashMap<String, YamlTask>>,
    #[serde(default = "default_as_false", alias = "buildable")]
    is_builder: bool,
    #[serde(rename = "meta")]
    _meta: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrVecString {
    Single(String),
    List(Vec<String>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct YamlModule {
    #[serde(default = "default_none", deserialize_with = "check_module_name")]
    name: Option<String>,
    context: Option<StringOrVecString>,
    help: Option<String>,
    depends: Option<Vec<StringOrMapVecString>>,
    selects: Option<Vec<StringOrMapVecString>>,
    uses: Option<Vec<String>>,
    provides: Option<Vec<String>>,
    provides_unique: Option<Vec<String>>,
    #[serde(alias = "disables")]
    conflicts: Option<Vec<String>>,
    #[serde(default = "default_as_false")]
    notify_all: bool,
    sources: Option<Vec<StringOrMapVecString>>,
    tasks: Option<HashMap<String, YamlTask>>,
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
    #[serde(rename = "meta")]
    _meta: Option<Value>,
}

impl YamlModule {
    fn default_binary() -> YamlModule {
        YamlModule {
            _is_binary: true,
            ..Self::default()
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
#[serde(deny_unknown_fields)]
struct YamlModuleEnv {
    local: Option<Env>,
    export: Option<Env>,
    global: Option<Env>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct YamlRule {
    pub name: String,
    pub cmd: String,

    pub help: Option<String>,

    #[serde(rename = "in")]
    pub in_: Option<String>,
    pub out: Option<String>,
    pub context: Option<String>,
    pub options: Option<HashMap<String, String>>,
    pub gcc_deps: Option<String>,
    pub rspfile: Option<String>,
    pub rspfile_content: Option<String>,
    pub pool: Option<String>,
    pub description: Option<String>,
    pub export: Option<Vec<StringOrMapString>>,

    #[serde(default = "default_as_false")]
    pub always: bool,

    #[serde(rename = "meta")]
    _meta: Option<Value>,
}

impl From<YamlRule> for Rule {
    //TODO: use deserialize_with as only the export field needs special handling
    fn from(yaml_rule: YamlRule) -> Self {
        Rule {
            always: yaml_rule.always,
            cmd: yaml_rule.cmd,
            context: yaml_rule.context,

            name: yaml_rule.name,
            help: yaml_rule.help,
            in_: yaml_rule.in_,
            out: yaml_rule.out,
            options: yaml_rule.options,
            gcc_deps: yaml_rule.gcc_deps,
            rspfile: yaml_rule.rspfile,
            rspfile_content: yaml_rule.rspfile_content,
            pool: yaml_rule.pool,
            description: yaml_rule.description,
            export: yaml_rule
                .export
                .map(|s| s.iter().map(|s| s.clone().into()).collect_vec()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct YamlTask {
    pub cmd: Vec<String>,
    pub help: Option<String>,
    pub required_vars: Option<Vec<String>>,
    pub required_modules: Option<Vec<String>>,
    pub export: Option<Vec<StringOrMapString>>,
    #[serde(default = "default_as_true")]
    pub build: bool,
    #[serde(default = "default_as_false")]
    pub ignore_ctrl_c: bool,
    #[serde(rename = "meta")]
    _meta: Option<Value>,
}

impl From<YamlTask> for Task {
    fn from(yaml_task: YamlTask) -> Self {
        Task {
            cmd: yaml_task.cmd,
            help: yaml_task.help,
            required_vars: yaml_task.required_vars,
            required_modules: yaml_task.required_modules,
            export: yaml_task
                .export
                .map(|s| s.iter().map(|s| s.clone().into()).collect_vec()),
            build: yaml_task.build,
            ignore_ctrl_c: yaml_task.ignore_ctrl_c,
        }
    }
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
        parsed.import_root.clone_from(&file_include.import_root);
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

pub fn load(
    filename: &Utf8Path,
    build_dir: &Utf8Path,
) -> Result<(ContextBag, FileTreeState, LoadStats)> {
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
            if let Some(includes) = &new.includes {
                let relpath = filename.parent().unwrap().to_path_buf();
                for filename in includes {
                    let filepath = Utf8Path::new(&relpath).join(filename);
                    filenames.insert(FileInclude::new(
                        filepath,
                        new.doc_idx,
                        new.import_root.clone(),
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
    ) -> Result<Module, Error> {
        let context_name = &context.name;
        let context_parent = match &context.parent {
            Some(x) => x.clone(),
            None => "default".to_string(),
        };

        let is_default = context_name.as_str() == "default";

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
                    if is_default {
                        None
                    } else {
                        Some(context_parent.clone())
                    },
                ),
                is_builder,
            )
            .with_context(|| format!("{:?}: adding context \"{}\"", &filename, &context_name))?;

        context_.help.clone_from(&context.help);
        context_.env.clone_from(&context.env);
        if let Some(rules) = &context.rules {
            context_.rules = Some(IndexMap::new());
            for rule in rules {
                let mut rule: Rule = rule.clone().into();
                rule.context = Some(context_name.clone());
                context_
                    .rules
                    .as_mut()
                    .unwrap()
                    .insert(rule.name.clone(), rule);
            }
        }
        context_.var_options.clone_from(&context.var_options);
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
            context_.tasks = Some(
                convert_tasks(tasks, &context_.env_early)
                    .with_context(|| format!("{:?} context \"{}\"", &filename, context.name))?,
            )
        }

        context_.apply_early_env()?;

        context_.defined_in = Some(filename.clone());

        // TODO(context-early-disables)
        context_.disable.clone_from(&context.disables);

        // Each Context has an associated module.
        // This holds:
        // - selects
        // - disables
        // TODO:
        // - env (in global env)
        // - rules
        // - tasks
        let module_name = Some(context_.module_name());
        let mut module = init_module(
            &module_name,
            Some(context_name),
            false,
            filename,
            import_root,
            None,
        );

        // collect context level "select:"
        if let Some(selects) = &context.selects {
            for dep_name in selects {
                // println!("- {}", dep_name);
                module.selects.push(dependency_from_string(dep_name));
            }
        }

        if let Some(disables) = context.disables.as_ref() {
            module.conflicts = Some(disables.clone());
        }

        // make context module depend on its parent's context module
        if !is_default {
            module
                .selects
                .push(Dependency::Hard(Context::module_name_for(&context_parent)));
        }

        Ok(module)
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
                relpath
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

        m.help.clone_from(&module.help);

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
            for dep_spec in selects {
                match dep_spec {
                    StringOrMapVecString::String(dep_name) => {
                        m.selects.push(dependency_from_string(dep_name));
                    }
                    StringOrMapVecString::Map(dep_map) => {
                        for (k, v) in dep_map {
                            for dep_name in v {
                                m.selects.push(dependency_from_string_if(dep_name, k));
                            }
                        }
                    }
                }
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
            m.add_conflicts(conflicts);
        }

        if let Some(provides) = &module.provides {
            m.add_provides(provides);
        }

        if let Some(provides_unique) = &module.provides_unique {
            // a "uniquely provided module" requires to be the only provider
            // for that module. think `provides_unique: [ libc ]`.
            // practically, it means adding to both "provides" and "conflicts"
            m.add_conflicts(provides_unique);
            m.add_provides(provides_unique);
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
                m.env_local.merge(local);
            }
            if let Some(export) = &env.export {
                m.env_export.merge(export);
            }
            if let Some(global) = &env.global {
                m.env_global.merge(global);
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
            m.blocklist.clone_from(&module.blocklist);
        }

        if let Some(defaults_allowlist) = &mut m.allowlist {
            if let Some(module_allowlist) = &module.allowlist {
                defaults_allowlist.append(&mut (module_allowlist.clone()));
            }
        } else {
            m.allowlist.clone_from(&module.allowlist);
        }

        let relpath = m.relpath.as_ref().unwrap().clone();

        m.download.clone_from(&module.download);
        let srcdir = if let Some(download) = &m.download {
            let srcdir = download.srcdir(build_dir, &m);
            let tagfile = download.tagfile(&srcdir);

            m.add_build_dep_file(&tagfile);

            // if a module has downloaded files, always consider it to be a
            // build dependency, as all dependees / users might include e.g.,
            // downloaded headers.
            m.is_build_dep = true;

            srcdir
        } else if relpath != "." {
            relpath.clone()
        } else {
            "".into()
        };

        m.build.clone_from(&module.build);
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

        // handle module tasks
        if let Some(tasks) = &module.tasks {
            m.tasks = convert_tasks(tasks, &m.env_early)
                .with_context(|| format!("{:?} module \"{}\"", &filename, m.name))?;

            // This makes the module provide_unique a marker module `::task::<task-name>`
            // for each task it defines, enabling the dependency resolver to sort
            // out duplicates.
            m.add_provides(tasks.keys().map(|name| format!("::task::{name}")));
            m.add_conflicts(tasks.keys().map(|name| format!("::task::{name}")));
        }

        if is_binary {
            m.env_global
                .insert("appdir".into(), EnvKey::Single(relpath.to_string()));
        }

        Ok(m)
    }

    // collect and convert contexts
    // this needs to be done before collecting modules, as that requires
    // contexts to be finalized.
    let mut context_modules = Vec::new();
    for data in &yaml_datas {
        for (list, is_builder) in [(&data.contexts, false), (&data.builders, true)].iter() {
            if let Some(context_list) = list {
                for context in context_list {
                    let module = convert_context(
                        context,
                        &mut contexts,
                        *is_builder | context.is_builder,
                        data.filename.as_ref().unwrap(),
                        &data.import_root,
                    )?;
                    context_modules.push(module);
                }
            }
        }
    }

    // after this, there's a default context, context relationships and envs have been set up.
    // modules can now be processed.
    contexts.finalize()?;

    // add the associated modules to their respective contexts
    for module in context_modules.drain(..) {
        contexts.add_module(module)?;
    }

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
                    .map(|context| match context {
                        StringOrVecString::List(_) => {
                            panic!("module defaults with context _list_")
                        }
                        StringOrVecString::Single(context) => context,
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
                    let module = YamlModule::default_binary();
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

    let parsing_time = start.elapsed();
    let start = Instant::now();

    // convert Utf8PathBufs to PathBufs
    // TODO: make treestate support camino Utf8PathBuf
    let filenames = filenames
        .drain(..)
        .map(|include| include.filename.into_std_path_buf())
        .collect_vec();

    let treestate = FileTreeState::new(filenames.iter());
    let stat_time = start.elapsed();

    let stats = LoadStats {
        parsing_time,
        stat_time,
        files: filenames.len(),
    };
    Ok((contexts, treestate, stats))
}

fn convert_tasks(
    tasks: &HashMap<String, YamlTask>,
    env: &Env,
) -> Result<HashMap<String, Task>, Error> {
    let flattened_env = env.flatten()?;
    tasks
        .iter()
        .map(|(name, task)| {
            let task = Task::from(task.clone());
            let task = task.with_env(&flattened_env);
            task.map(|task| (name.clone(), task))
                .with_context(|| format!("task \"{}\"", name.clone()))
        })
        .collect::<Result<_, _>>()
}
