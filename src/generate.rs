//! This module is responsible for generating the .ninja files.
//! It expects data structures as created by the data module.

use core::hash::Hash;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::hash::Hasher;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use nested_env::EnvKey;
//use rayon::iter::ParallelBridge;
use rayon::prelude::*;

use super::{
    data::{load, FileTreeState},
    download, nested_env,
    nested_env::{Env, IfMissing},
    ninja::{NinjaBuildBuilder, NinjaRule, NinjaRuleBuilder, NinjaRuleDeps},
    BlockAllow, Build, Context, ContextBag, Dependency, Module, Task,
};

#[derive(Deserialize, Serialize)]
pub struct BuildInfo {
    pub tasks: IndexMap<String, Task>,
    pub out: PathBuf,
}

pub type BuildInfoList = Vec<(String, String, BuildInfo)>;

#[derive(Clone, Deserialize, Serialize)]
pub enum GenerateMode {
    Global,
    Local(PathBuf),
}

pub fn get_ninja_build_file(build_dir: &Path, mode: &GenerateMode) -> PathBuf {
    match mode {
        GenerateMode::Global => build_dir.join("build-global.ninja"),
        GenerateMode::Local(_) => build_dir.join("build-local.ninja"),
    }
}

fn relroot(relpath: &Path) -> PathBuf {
    let components = relpath.components().count();
    if components == 0 {
        "./".into()
    } else {
        let mut res = PathBuf::new();
        for _ in 0..components {
            res.push("..");
        }
        res
    }
}

pub fn generate(
    project_root: &Path,
    build_dir: &Path,
    mode: GenerateMode,
    builders: Selector,
    apps: Selector,
    select: Option<Vec<String>>,
    disable: Option<Vec<String>>,
    cli_env: Option<&Env>,
) -> Result<GenerateResult> {
    let start = Instant::now();

    // convert CLI --select strings to Vec<Dependency>
    let select = select.map_or(None, |mut vec| {
        Some(
            vec.drain(..)
                .map(|dep_name| crate::data::dependency_from_string(&dep_name))
                .collect_vec(),
        )
    });

    match GenerateResult::from_cache(
        project_root,
        build_dir,
        &mode,
        &builders,
        &apps,
        select.as_ref(),
        disable.as_ref(),
        &cli_env,
    ) {
        Ok(cached) => {
            println!("laze: reading cache took {:?}.", start.elapsed());
            return Ok(cached);
        }
        Err(x) => match x {
            _ => println!("laze: reading cache: {}", x),
        },
    }

    let (contexts, treestate) = load(project_root, build_dir)?;

    std::fs::create_dir_all(&build_dir)?;
    let mut ninja_build_file = std::io::BufWriter::new(std::fs::File::create(
        get_ninja_build_file(build_dir, &mode).as_path(),
    )?);

    ninja_build_file
        .write_all(format!("builddir = {}\n", build_dir.to_str().unwrap()).as_bytes())?;

    // add phony helper
    ninja_build_file.write_all(b"build ALWAYS: phony\n")?;

    let start = Instant::now();

    let mut laze_env = im::HashMap::new();
    laze_env.insert(
        "in".to_string(),
        nested_env::EnvKey::Single("\\${in}".to_string()),
    );
    laze_env.insert(
        "out".to_string(),
        nested_env::EnvKey::Single("\\${out}".to_string()),
    );
    laze_env.insert(
        "build-dir".to_string(),
        nested_env::EnvKey::Single(build_dir.to_string_lossy().into()),
    );

    let laze_env = laze_env;

    let selected_builders = match &builders {
        Selector::All => contexts.builders_vec(),
        Selector::Some(builders) => contexts.builders_by_name(builders),
    };

    // get all "binary" modules
    let bins = contexts
        .contexts
        .iter()
        .flat_map(|ctx| ctx.modules.iter())
        .filter(|(_, module)| module.is_binary);

    // filter selected apps, if specified
    // also filter by apps in the start folder, if not in global mode
    let bins = bins.filter(|(_, module)| {
        if let Selector::Some(apps) = &apps {
            if let None = apps.get(&module.name[..]) {
                return false;
            }
        }
        if let GenerateMode::Local(start_dir) = &mode {
            if module.relpath.as_ref().unwrap() != start_dir {
                return false;
            }
        }
        true
    });

    // create (builder, bin) tuples
    let builder_bin_tuples = selected_builders.iter().cartesian_product(bins);

    // actually configure builds
    let mut builds = builder_bin_tuples
        .collect::<Vec<_>>()
        .par_iter()
        // `.par_bridge()` instead of `collect()+par_iter()` yields slight (1%) configure time
        // speedup, at the price of changing the order of build rules. not worth losing
        // reproducible output.
        .filter_map(|(builder, (_, bin))| {
            match configure_build(
                bin,
                &contexts,
                builder,
                &laze_env,
                select.as_ref(),
                disable.as_ref(),
                &cli_env,
            )
            .unwrap()
            {
                Some((build_info, ninja_entries)) => Some((
                    builder.name.clone(),
                    bin.name.clone(),
                    build_info,
                    ninja_entries,
                )),
                _ => None,
            }
        })
        .collect::<Vec<(String, String, BuildInfo, IndexSet<String>)>>();

    let mut combined_ninja_entries = IndexSet::new();
    let builds = builds
        .drain(..)
        .map(|(builder, bin, build_info, ninja_entries)| {
            combined_ninja_entries.extend(ninja_entries);
            (builder, bin, build_info)
        })
        .collect::<Vec<_>>();

    for entry in combined_ninja_entries {
        ninja_build_file.write_all(entry.as_bytes())?;
    }

    let num_built = builds.len();
    println!(
        "configured {} builds (took {:?}).",
        num_built,
        start.elapsed()
    );

    let result = GenerateResult::new(
        mode, builders, apps, builds, treestate, select, disable, cli_env,
    );
    let start = Instant::now();
    result.to_cache(&build_dir)?;
    println!("laze: writing cache took {:?}.", start.elapsed());
    Ok(result)
}

// This function "renders" a specific app/builder pair, if dependencies
// and block/allowlists allow it.
//
// TODO: configure_build() is approaching 300 LoC.  it should be split up.
fn configure_build(
    binary: &Module,
    contexts: &ContextBag,
    builder: &Context,
    laze_env: &Env,
    select: Option<&Vec<Dependency>>,
    disable: Option<&Vec<String>>,
    cli_env: &Option<&Env>,
) -> Result<Option<(BuildInfo, IndexSet<String>)>> {
    if !match contexts.is_allowed(builder, &binary.blocklist, &binary.allowlist) {
        BlockAllow::Allowed => true,
        BlockAllow::Blocked => {
            println!("app {}: builder {} blocklisted", binary, builder.name);
            false
        }
        BlockAllow::BlockedBy(index) => {
            println!(
                "app {}: parent {} of builder {} blocklisted",
                binary.name,
                contexts.context_by_id(index).name,
                builder.name
            );
            false
        }
        BlockAllow::AllowedBy(index) => {
            println!(
                "app {}: parent {} of builder {} allowlisted",
                binary.name,
                contexts.context_by_id(index).name,
                builder.name
            );
            true
        }
    } {
        return Ok(None);
    }

    println!("configuring {} for {}", binary.name, builder.name);

    /* create build instance (binary A for builder X) */
    let build = Build::new(binary, builder, contexts, select);

    // get build dir from laze_env
    let build_dir = match laze_env.get("build-dir").unwrap() {
        nested_env::EnvKey::Single(path) => PathBuf::from(path),
        _ => unreachable!(),
    };

    // collect disabled modules from app and build context
    let mut disabled_modules = build.build_context.collect_disabled_modules(&contexts);
    if let Some(disable) = &binary.disable {
        for entry in disable {
            disabled_modules.insert(entry.clone());
        }
    }

    // collect modules disabled on CLI
    if let Some(disable) = disable {
        for entry in disable {
            disabled_modules.insert(entry.clone());
        }
    }

    /* resolve all dependency names to specific modules.
     * this also determines if all dependencies are met */
    let modules = match build.resolve_selects(&disabled_modules) {
        Err(e) => {
            println!("error: {}", e);
            return Ok(None);
        }
        Ok(val) => val,
    };

    /* collect build context rules */
    let mut rules = IndexMap::new();
    let rules = build.build_context.collect_rules(&contexts, &mut rules);
    let merge_opts = &builder.var_options;

    // create initial build context global env.
    let mut global_env =
        nested_env::merge(laze_env.clone(), build.build_context.env.clone().unwrap());

    /* import global module environments into global build context env */
    for (_, module) in modules.iter().rev() {
        global_env = nested_env::merge(global_env, module.env_global.clone());
    }

    // insert global "relpath"
    // this will be overridden by each module's environment.
    // inserting it here (to the relpath of the app)
    // makes it available to the linking step and tasks.
    global_env.insert(
        "relpath".into(),
        EnvKey::Single(binary.relpath.as_ref().unwrap().to_str().unwrap().into()),
    );

    global_env.insert(
        "relroot".into(),
        EnvKey::Single(
            relroot(binary.relpath.as_ref().unwrap())
                .to_str()
                .unwrap()
                .into(),
        ),
    );

    // if provided, merge CLI env overrides
    if let Some(cli_env) = *cli_env {
        global_env = nested_env::merge(global_env, cli_env.clone());
    }

    let tmp = global_env.clone();
    let out_string = String::from("out");
    let mut global_env_flattened = nested_env::flatten_with_opts_option(&tmp, merge_opts.as_ref());

    let bindir = nested_env::expand("${bindir}", &global_env_flattened, IfMissing::Empty).unwrap();
    let bindir = PathBuf::from(bindir);

    /* build application file name */
    let out_elf = Path::new(&bindir).join(&binary.name).with_extension("elf");

    let mut objdir = build_dir.clone();
    objdir.push("objects");

    // vector collecting objects, later used as linking inputs
    let mut objects = Vec::new();

    /* set containing ninja build or rule blocks */
    let mut ninja_entries = IndexSet::new();

    /* now handle each module */
    for (_, module) in modules.iter() {
        // handle possible remote sources
        let download_rules = download::handle_module(module, &build_dir, &rules)?;

        if let Some(mut download_rules) = download_rules {
            ninja_entries.extend(download_rules.drain(..));
        }

        // "srcdir" is either the folder of laze.yml that defined this module,
        // *or* if it was downloaded, the download folder.
        // This is populated in data.rs, do unwrap() always succeeds.
        let srcdir = module.srcdir.as_ref().unwrap();

        /* build final module env */
        let module_env = module.build_env(&global_env, &modules);

        let flattened_env = nested_env::flatten_with_opts_option(&module_env, merge_opts.as_ref());
        //println!("{:#?}", builder.var_options);

        // add optional sources, if needed
        let mut optional_sources = Vec::new();
        if let Some(optional_sources_map) = &module.sources_optional {
            for (k, v) in optional_sources_map {
                if modules.contains_key(k) {
                    for entry in v {
                        optional_sources.push(entry.clone());
                    }
                }
            }
        }

        // map extension -> rule for this module
        let mut module_rules: IndexMap<String, NinjaRule> = IndexMap::new();

        /* apply rules to sources */
        for source in module.sources.iter().chain(optional_sources.iter()) {
            let ext = Path::new(&source)
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();

            // This block finds a rule for this source file's extension
            // (e.g., .c -> CC).
            // If there is one, use it, otherwise create a new one from the
            // context rules, applying this module's env.
            module_rules.entry(ext.into()).or_insert({
                let rule = match rules.get(ext.into()) {
                    Some(rule) => rule,
                    None => {
                        return Err(anyhow!(
                            "no rule found for \"{}\" of module \"{}\" (from {})",
                            source,
                            module.name,
                            module.defined_in.as_ref().unwrap().to_string_lossy(),
                        ))
                    }
                };
                let expanded =
                    nested_env::expand(&rule.cmd, &flattened_env, IfMissing::Empty).unwrap();

                let rule = NinjaRuleBuilder::default()
                    .name(Cow::from(&rule.name))
                    .description(Some(Cow::from(&rule.name)))
                    .rspfile(rule.rspfile.as_deref().map(Cow::from))
                    .rspfile_content(rule.rspfile_content.as_deref().map(Cow::from))
                    .pool(rule.pool.as_deref().map(Cow::from))
                    .command(expanded)
                    .deps(match &rule.gcc_deps {
                        None => NinjaRuleDeps::None,
                        Some(s) => NinjaRuleDeps::GCC(s.into()),
                    })
                    .build()
                    .unwrap()
                    .named();
                ninja_entries.insert(format!("{}", &rule));
                rule
            });
        }

        // this ugly block collects all "build_deps" entries (which are Strings
        // converted from Paths) into Vec<Cow<Path>> as needed by NinjaBuildBuilder's "deps()"
        // method.
        let build_deps = if let Some(build_deps) = module_env.get("build_deps".into()) {
            if let nested_env::EnvKey::List(list) = build_deps {
                Some(
                    list.iter()
                        .map(|x| Cow::from(PathBuf::from(x)))
                        .collect_vec(),
                )
            } else {
                unreachable!();
            }
        } else {
            None
        };

        // now for each source file,
        for source in module.sources.iter().chain(optional_sources.iter()) {
            // 1. determine full file path (relative to project root)
            let mut srcpath = srcdir.clone();
            srcpath.push(source);

            // 2. find ninja rule by lookup of the source file's extension
            let ext = Path::new(&source)
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();

            let rule = rules.get(ext.into()).unwrap();

            let ninja_rule = module_rules.get(ext.into()).unwrap();
            let rule_hash = ninja_rule.get_hash();

            // 3. determine output path (e.g., name of C object file)
            let out =
                srcpath.with_extension(format!("{}.{}", rule_hash, &rule.out.as_ref().unwrap()));

            let mut object = objdir.clone();
            object.push(out);

            // 4. render ninja "build:" snippet and add to this build's
            // ninja statement set
            let build = NinjaBuildBuilder::default()
                .rule(&*ninja_rule.name)
                .in_single(Cow::from(&srcpath))
                .out(object.as_path())
                //.deps(build_deps.clone())
                .build()
                .unwrap();

            ninja_entries.insert(format!("{}", build));

            // 5. store the output in this build's output list
            objects.push(object);

            // 6. optionally create dependency to the download / patch step
            if let Some(build_deps) = &build_deps {
                let build = NinjaBuildBuilder::default()
                    .rule("phony")
                    .in_vec(build_deps.clone())
                    .out(Cow::from(&srcpath))
                    .build()
                    .unwrap();

                ninja_entries.insert(format!("{}", build));
            }
        }
    }

    // NinjaBuildBuilder expects a Vec<&Path>, but the loop above creates a Vec<PathBuf>.
    // thus, convert.
    let objects: Vec<_> = objects.iter().map(|pathbuf| Cow::from(pathbuf)).collect();

    // linking
    {
        let link_rule = match rules.values().find(|rule| rule.name == "LINK") {
            Some(x) => x,
            // returning an error here won't show, just not configure the build
            // None => return Err(anyhow!("missing LINK rule for builder {}", builder.name)),
            None => panic!("missing LINK rule for builder {}", builder.name),
        };
        let expanded =
            nested_env::expand(&link_rule.cmd, &global_env_flattened, IfMissing::Empty).unwrap();

        let ninja_link_rule = NinjaRuleBuilder::default()
            .name(&link_rule.name)
            .description(Some(Cow::from(&link_rule.name)))
            .command(expanded)
            .rspfile(link_rule.rspfile.as_deref().map(Cow::from))
            .rspfile_content(link_rule.rspfile_content.as_deref().map(Cow::from))
            .pool(link_rule.pool.as_deref().map(Cow::from))
            .build()
            .unwrap()
            .named();

        // build ninja link target
        let ninja_link_build = NinjaBuildBuilder::default()
            .rule(&*ninja_link_rule.name)
            .in_vec(objects)
            .out(out_elf.as_path())
            .always(link_rule.always)
            .build()
            .unwrap();

        ninja_entries.insert(format!("{}", ninja_link_rule));
        ninja_entries.insert(format!("{}", ninja_link_build));
    }

    // collect tasks
    global_env_flattened.insert(&out_string, String::from(out_elf.to_str().unwrap()));
    let tasks = build
        .build_context
        .collect_tasks(&contexts, &global_env_flattened);

    Ok(Some((
        BuildInfo {
            tasks,
            out: out_elf,
        },
        ninja_entries,
    )))
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Selector {
    All,
    Some(IndexSet<String>),
}

impl Selector {
    pub fn is_superset(&self, other: &Selector) -> bool {
        match self {
            Selector::All => true,
            Selector::Some(set) => match other {
                Selector::All => false,
                Selector::Some(other_set) => set.is_superset(other_set),
            },
        }
    }

    pub fn selects(&self, value: &String) -> bool {
        if let Selector::Some(set) = self {
            set.contains(value)
        } else {
            true
        }
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::All => write!(f, "all")?,
            Selector::Some(list) => {
                let mut it = list.iter();
                if let Some(entry) = it.next() {
                    write!(f, "{}", entry)?;
                    for entry in it {
                        write!(f, ", {}", entry)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct GenerateResult {
    pub mode: GenerateMode,
    pub builders: Selector,
    pub apps: Selector,
    pub build_infos: Vec<(String, String, BuildInfo)>,

    select: Option<Vec<Dependency>>,
    disable: Option<Vec<String>>,
    cli_env_hash: u64,
    treestate: FileTreeState,
}

impl GenerateResult {
    pub fn new(
        mode: GenerateMode,
        builders: Selector,
        apps: Selector,
        build_infos: BuildInfoList,
        treestate: FileTreeState,
        select: Option<Vec<Dependency>>,
        disable: Option<Vec<String>>,
        cli_env: Option<&Env>,
    ) -> GenerateResult {
        GenerateResult {
            mode,
            builders,
            apps,
            build_infos,
            select,
            disable,
            cli_env_hash: cli_env.map_or(0, calculate_hash),
            treestate,
        }
    }

    fn cache_file(build_dir: &Path, mode: &GenerateMode) -> PathBuf {
        match mode {
            GenerateMode::Global => build_dir.join("laze-cache-global.bincode"),
            GenerateMode::Local(_) => build_dir.join("laze-cache-local.bincode"),
        }
    }

    pub fn from_cache(
        _project_root: &Path,
        build_dir: &Path,
        mode: &GenerateMode,
        builders: &Selector,
        apps: &Selector,
        select: Option<&Vec<Dependency>>,
        disable: Option<&Vec<String>>,
        cli_env: &Option<&Env>,
    ) -> Result<GenerateResult> {
        let file = Self::cache_file(build_dir, mode);
        let file = File::open(file)?;
        let res: GenerateResult = bincode::deserialize_from(file)?;
        if !res.builders.is_superset(builders) {
            return Err(anyhow!("builders don't match"));
        }
        if !res.apps.is_superset(apps) {
            return Err(anyhow!("apps don't match"));
        }
        if let GenerateMode::Local(path) = mode {
            if let GenerateMode::Local(cached_path) = &res.mode {
                if path != cached_path {
                    return Err(anyhow!("local paths don't match"));
                }
            }
        }
        if !res.select.as_ref().eq(&select) {
            return Err(anyhow!("CLI selects don't match"));
        }
        if !res.disable.as_ref().eq(&disable) {
            return Err(anyhow!("CLI disables don't match"));
        }
        if res.cli_env_hash != cli_env.map_or(0, calculate_hash) {
            return Err(anyhow!("laze: CLI env doesn't match"));
        }
        if res.treestate.has_changed() {
            return Err(anyhow!("laze: build files have changed"));
        }
        Ok(res)
    }

    pub fn to_cache(&self, build_dir: &Path) -> std::result::Result<(), Box<bincode::ErrorKind>> {
        let file = Self::cache_file(build_dir, &self.mode);
        let file = File::create(file)?;
        bincode::serialize_into(file, self)
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
