use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use super::{
    data::{load, FileTreeState},
    nested_env,
    nested_env::{Env, IfMissing},
    ninja::{NinjaBuildBuilder, NinjaRule, NinjaRuleBuilder, NinjaRuleDeps, NinjaWriter},
    BlockAllow, Build, Context, ContextBag, Module, Task,
};

pub struct BuildInfo {
    pub tasks: IndexMap<String, Task>,
}

pub enum GenerateMode<'a> {
    Global,
    Local(&'a Path),
}

pub fn generate(
    project_root: &Path,
    build_dir: &Path,
    mode: GenerateMode,
    builders: &IndexSet<&str>,
    apps: &IndexSet<&str>,
) -> Result<Vec<(String, String, BuildInfo)>> {
    if let Ok(cached) = generate_cache(project_root, build_dir, &mode, builders, apps) {
        return Ok(cached);
    }

    let start = Instant::now();
    let (contexts, treestate) = load(project_root)?;

    std::fs::create_dir_all(&build_dir)?;
    let mut ninja_writer = NinjaWriter::new(build_dir.join("build.ninja").as_path()).unwrap();

    fn configure_build(
        binary: &Module,
        contexts: &ContextBag,
        builder: &Context,
        ninja_writer: &mut NinjaWriter,
        laze_env: &Env,
    ) -> Result<Option<BuildInfo>> {
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
        let build = Build::new(binary, builder, contexts);

        /* create initial build context global env.
         * Unfortunately we need to create a copy as we cannot get a mutable
         * reference to build_context.env. */
        let mut global_env = Env::new();
        nested_env::merge(&mut global_env, &laze_env);
        nested_env::merge(&mut global_env, &build.build_context.env.as_ref().unwrap());

        // collect disabled modules from app and build context
        let mut disabled_modules = build.build_context.collect_disabled_modules(&contexts);
        if let Some(disable) = &binary.disable {
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

        /* import global module environments into global build context env */
        for (_, module) in modules.iter().rev() {
            nested_env::merge(&mut global_env, &module.env_global);
        }

        let mut app_builds = Vec::new();

        /* now handle each module */
        for (_, module) in modules.iter() {
            /* build final module env */
            let module_env = module.build_env(&global_env, &modules);

            /* add escaped ${in} and ${out}, create env for the build rules */
            let mut rule_env = Env::new();
            nested_env::merge(&mut rule_env, &module_env);
            let flattened_env =
                nested_env::flatten_with_opts_option(&rule_env, merge_opts.as_ref());
            //println!("{:#?}", builder.var_options);

            let mut module_rules: IndexMap<String, NinjaRule> = IndexMap::new();
            let mut module_builds = Vec::new();

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

            /* apply rules to sources */
            for source in module.sources.iter().chain(optional_sources.iter()) {
                let ext = Path::new(&source)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap();

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

                    NinjaRuleBuilder::default()
                        .name(&*rule.name)
                        .description(&*rule.name)
                        .command(&*expanded)
                        .deps(match &rule.gcc_deps {
                            None => NinjaRuleDeps::None,
                            Some(s) => NinjaRuleDeps::GCC(s.clone()),
                        })
                        .build()
                        .unwrap()
                });
            }

            let srcdir = module.defined_in.as_ref().unwrap().parent().unwrap();
            for source in module.sources.iter().chain(optional_sources.iter()) {
                let ext = Path::new(&source)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap();

                let mut srcpath = srcdir.to_path_buf();
                srcpath.push(source);
                let rule = rules.get(ext.into()).unwrap();
                let ninja_rule = module_rules.get(ext.into()).unwrap();
                let out = srcpath.with_extension(&rule.out.as_ref().unwrap());
                module_builds.push((ninja_rule.clone(), srcpath, out));
            }

            app_builds.append(&mut module_builds);
        }

        let mut objects = Vec::new();

        let relpath = binary.relpath.as_ref().unwrap();
        global_env.insert(
            "relpath".to_string(),
            nested_env::EnvKey::Single(String::from(relpath.to_str().unwrap())),
        );

        let (ninja_link_rule, bindir) = {
            let link_rule = match rules.values().find(|rule| rule.name == "LINK") {
                Some(x) => x,
                // returning an error here won't show, just not configure the build
                // None => return Err(anyhow!("missing LINK rule for builder {}", builder.name)),
                None => panic!("missing LINK rule for builder {}", builder.name),
            };
            let mut link_env = Env::new();
            nested_env::merge(&mut link_env, &global_env);
            let flattened_env = nested_env::flatten(&link_env);
            let expanded =
                nested_env::expand(&link_rule.cmd, &flattened_env, IfMissing::Empty).unwrap();

            (
                NinjaRuleBuilder::default()
                    .name(&*link_rule.name)
                    .description(&*link_rule.name)
                    .command(&*expanded)
                    .build()
                    .unwrap(),
                nested_env::expand("${bindir}", &flattened_env, IfMissing::Empty).unwrap(),
            )
        };

        let bindir = PathBuf::from(bindir);
        // write compile rules & builds
        for (rule, source, out) in &app_builds {
            let rule_name = ninja_writer.write_rule_dedup(rule).unwrap();
            let mut object = bindir.clone();
            object.push(out);

            let build = NinjaBuildBuilder::default()
                .rule(&*rule_name)
                .in_single(source.as_path())
                .out(object.as_path())
                .build()
                .unwrap();
            ninja_writer.write_build(&build).unwrap();

            objects.push(object);
        }

        /* build application file name */
        let mut out_elf = bindir.clone();
        out_elf.push(&binary.name);
        let out_elf = out_elf.with_extension("elf");

        // write linking rule & build
        let link_rule_name = ninja_writer.write_rule_dedup(&ninja_link_rule).unwrap();

        // NinjaBuildBuilder expects a Vec<&Path>, but the loop above creates a Vec<PathBuf>.
        // thus, convert.
        let objects: Vec<&Path> = objects.iter().map(|pathbuf| pathbuf.as_path()).collect();

        // build ninja link target
        let ninja_link_build = NinjaBuildBuilder::default()
            .rule(&*link_rule_name)
            .in_vec(objects)
            .out(out_elf.as_path())
            .build()
            .unwrap();

        ninja_writer.write_build(&ninja_link_build).unwrap();

        // collect tasks
        let mut task_env = Env::new();
        nested_env::merge(&mut task_env, &global_env);
        task_env.insert(
            "out".into(),
            nested_env::EnvKey::Single(String::from(out_elf.to_str().unwrap())),
        );
        let flattened_task_env = nested_env::flatten(&task_env);
        let tasks = build
            .build_context
            .collect_tasks(&contexts, &flattened_task_env);

        Ok(Some(BuildInfo { tasks }))
    }

    let mut laze_env = HashMap::new();
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

    let selected_builders = if builders.is_empty() {
        contexts.builders_vec()
    } else {
        contexts.builders_by_name(builders)
    };

    let all_apps = apps.is_empty();

    // get all "binary" modules
    let bins = contexts
        .contexts
        .iter()
        .flat_map(|ctx| ctx.modules.iter())
        .filter(|(_, module)| module.is_binary);

    // filter selected apps, if specified
    // also filter by apps in the start folder, if not in global mode
    let bins = bins.filter(|(_, module)| {
        if !all_apps {
            if let None = apps.get(&module.name[..]) {
                return false;
            }
        }
        if let GenerateMode::Local(start_dir) = mode {
            if module.relpath.as_ref().unwrap() != start_dir {
                return false;
            }
        }
        true
    });

    // create (builder, bin) tuples
    let builder_bin_tuples = selected_builders.iter().cartesian_product(bins);

    // actually configure builds
    let builds = builder_bin_tuples
        .filter_map(|(builder, (_, bin))| {
            match configure_build(bin, &contexts, builder, &mut ninja_writer, &laze_env).ok()? {
                Some(build_info) => Some((builder.name.clone(), bin.name.clone(), build_info)),
                _ => None,
            }
        })
        .collect::<Vec<(String, String, BuildInfo)>>();

    let num_built = builds.len();
    println!(
        "configured {} builds (took {:?}).",
        num_built,
        start.elapsed()
    );

    Ok(builds)
}

fn generate_cache(
    project_root: &Path,
    build_dir: &Path,
    mode: &GenerateMode,
    builders: &IndexSet<&str>,
    apps: &IndexSet<&str>,
) -> Result<Vec<(String, String, BuildInfo)>> {
    Err(anyhow!("cache not implemented"))
}
