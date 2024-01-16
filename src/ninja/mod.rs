#![allow(clippy::upper_case_acronyms)]

use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::process::{Command, ExitStatus, Stdio};

use camino::{Utf8Path, Utf8PathBuf};
use indexmap::IndexMap;

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Default)]
pub enum NinjaRuleDeps {
    #[default]
    None,
    GCC(String),
}



#[derive(Builder, Debug, PartialEq, Eq, Clone)]
#[builder(setter(into))]
pub struct NinjaRule<'a> {
    pub name: Cow<'a, str>,
    command: Cow<'a, str>,
    description: Option<Cow<'a, str>>,
    #[builder(setter(into, strip_option), default = "None")]
    env: Option<&'a IndexMap<String, String>>,
    #[builder(default = "NinjaRuleDeps::None")]
    deps: NinjaRuleDeps,
    #[builder(default = "None")]
    rspfile: Option<Cow<'a, str>>,
    #[builder(default = "None")]
    rspfile_content: Option<Cow<'a, str>>,
    #[builder(default = "None")]
    pool: Option<Cow<'a, str>>,
    #[builder(default = "false")]
    pub always: bool,
}

impl<'a> fmt::Display for NinjaRule<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rule {}\n  command = {}\n", self.name, self.command)?;

        if let Some(description) = &self.description {
            writeln!(f, "  description = {description}")?;
        }

        if let NinjaRuleDeps::GCC(depfile) = &self.deps {
            write!(f, "  deps = gcc\n  depfile = {depfile}\n")?;
        }

        if let Some(rspfile) = &self.rspfile {
            writeln!(f, "  rspfile = {rspfile}")?;
        }

        if let Some(rspfile_content) = &self.rspfile_content {
            writeln!(f, "  rspfile_content = {rspfile_content}")?;
        }

        if let Some(pool) = &self.pool {
            writeln!(f, "  pool = {pool}")?;
        }

        writeln!(f)?;

        Ok(())
    }
}

impl<'a> NinjaRule<'a> {
    pub fn get_hash(&self, extra: Option<u64>) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        if let Some(extra) = extra {
            s.write_u64(extra);
        }
        s.finish()
    }

    pub fn get_hashed_name(&self, hash: u64) -> String {
        let mut name = String::from(self.name.clone());
        name.push_str(&format!("_{}", hash));
        name
    }

    // pub fn named_with_extra(mut self, extra: Option<u64>) -> NinjaRule<'a> {
    //     let name = self.get_hashed_name(self.get_hash(extra));
    //     self.name = Cow::from(name);
    //     self
    // }

    pub fn named(mut self) -> NinjaRule<'a> {
        let name = self.get_hashed_name(self.get_hash(None));
        self.name = Cow::from(name);
        self
    }
}

impl<'a> Hash for NinjaRule<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.command.hash(state);
        self.description.hash(state);
        self.rspfile.hash(state);
        self.rspfile_content.hash(state);
        match &self.deps {
            NinjaRuleDeps::None => (),
            NinjaRuleDeps::GCC(s) => s.hash(state),
        };
    }
}

#[derive(Builder, Debug)]
#[builder(setter(into))]
pub struct NinjaBuild<'a> {
    rule: Cow<'a, str>,

    #[builder(setter(strip_option), default = "None")]
    inputs: Option<Vec<Cow<'a, Utf8Path>>>,
    outs: Vec<Cow<'a, Utf8Path>>,

    #[builder(default = "None")]
    deps: Option<Vec<Cow<'a, Utf8Path>>>,

    #[builder(setter(into, strip_option), default = "None")]
    env: Option<&'a IndexMap<String, String>>,

    #[builder(default = "false")]
    always: bool,
    //   deps: NinjaRuleDeps,
}

impl<'a> NinjaBuildBuilder<'a> {
    pub fn out<I>(&mut self, out: I) -> &mut Self
    where
        I: Into<Cow<'a, Utf8Path>>,
    {
        if let Some(outs) = self.outs.as_mut() {
            outs.push(out.into());
        } else {
            self.outs = Some(vec![out.into()]);
        }
        self
    }

    pub fn input<I>(&mut self, input: I) -> &mut Self
    where
        I: Into<Cow<'a, Utf8Path>>,
    {
        if let Some(inputs) = self.inputs.as_mut() {
            if let Some(inputs) = inputs {
                inputs.push(input.into());
            } else {
                *inputs = Some(vec![input.into()]);
            }
        } else {
            self.inputs = Some(Some(vec![input.into()]));
        }
        self
    }

    pub fn with_rule(&mut self, rule: &'a NinjaRule) -> &mut Self {
        self.rule(&*rule.name).always(rule.always)
    }

    pub fn from_rule(rule: &'a NinjaRule) -> Self {
        let mut res = Self::default();
        res.with_rule(rule);
        res
    }
}

impl<'a> fmt::Display for NinjaBuild<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "build")?;

        for out in &self.outs {
            write!(f, " {out}")?;
        }

        write!(f, ": $\n    {}", self.rule)?;

        if let Some(inputs) = &self.inputs {
            for path in inputs {
                write!(f, " $\n    {path}")?;
            }
        }

        if self.deps.is_some() || self.always {
            write!(f, " $\n    |")?;
            if let Some(list) = &self.deps {
                for entry in list {
                    write!(f, " $\n    {entry}")?;
                }
            }
            if self.always {
                write!(f, " $\n    ALWAYS")?;
            }
        }
        writeln!(f)?;

        if let Some(env) = self.env {
            for (k, v) in env {
                writeln!(f, "  {k} = {v}")?;
            }
        }

        writeln!(f)
    }
}

// pub struct NinjaWriter {
//     pub file: BufWriter<File>,
//     pub rules: HashSet<u64>,
// }

// impl NinjaWriter {
//     pub fn new(path: &Utf8Path) -> std::io::Result<NinjaWriter> {
//         Ok(NinjaWriter {
//             file: BufWriter::new(File::create(path)?),
//             rules: HashSet::new(),
//         })
//     }

//     pub fn write_rule(&mut self, rule: &NinjaRule) -> std::io::Result<()> {
//         self.file.write_all(format!("{}", rule).as_bytes())
//     }

//     pub fn write_rule_dedup(&mut self, rule: &NinjaRule) -> std::io::Result<String> {
//         let rule_hash = rule.get_hash();
//         let name = rule.get_hashed_name(rule_hash);

//         if self.rules.insert(rule_hash) {
//             let mut named = rule.clone();
//             named.name = Cow::from(&name);
//             self.write_rule(&named)?;
//         }

//         Ok(name)
//     }

//     pub fn write_var(&mut self, var: &str, val: &str) -> std::io::Result<()> {
//         self.file
//             .write_all(format!("{} = {}\n", var, val).as_bytes())
//     }

//     pub fn write_build(&mut self, build: &NinjaBuild) -> std::io::Result<()> {
//         self.file.write_all(format!("{}", build).as_bytes())
//     }
// }

#[derive(Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct NinjaToolBase<'a> {
    #[builder(setter(into), default = "Utf8Path::new(\"ninja\")")]
    binary: &'a Utf8Path,

    #[builder(setter(into), default = "Utf8Path::new(\"build.ninja\")")]
    build_file: &'a Utf8Path,

    args: Vec<String>,
}

impl<'a> NinjaToolBaseBuilder<'a> {
    pub fn arg(&mut self, arg: String) -> &mut Self {
        if let Some(args) = self.args.as_mut() {
            args.push(arg);
        } else {
            self.args = Some(vec![arg]);
        }
        self
    }
}

impl<'a> NinjaToolBase<'a> {
    pub fn get_command(&self) -> Command {
        let mut cmd = Command::new(self.binary);
        cmd.arg("-f").arg(self.build_file);
        cmd.args(&self.args);
        cmd
    }
}

pub fn generate_compile_commands(
    build_file: &Utf8Path,
    target: &Utf8Path,
) -> Result<ExitStatus, std::io::Error> {
    let mut cmd = NinjaToolBaseBuilder::default()
        .build_file(build_file)
        .arg("-t".into())
        .arg("compdb".into())
        .build()
        .unwrap()
        .get_command();

    let target = std::fs::File::create(target)?;
    cmd.stdout(Stdio::from(target));
    cmd.spawn()?.wait()
}

#[derive(Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct NinjaCmd<'a> {
    #[builder(setter(into), default = "Utf8Path::new(\"ninja\")")]
    pub binary: &'a Utf8Path,

    #[builder(setter(into), default = "Utf8Path::new(\"build.ninja\")")]
    build_file: &'a Utf8Path,

    #[builder(default = "false")]
    verbose: bool,

    #[builder(default = "None")]
    targets: Option<Vec<Utf8PathBuf>>,

    #[builder(default = "None")]
    jobs: Option<usize>,
}

impl<'a> NinjaCmd<'a> {
    pub fn run(&self) -> std::io::Result<ExitStatus> {
        let mut cmd = Command::new(self.binary);
        cmd.arg("-f").arg(self.build_file);

        if self.verbose {
            cmd.arg("-v");
        }

        if let Some(jobs) = self.jobs {
            cmd.arg("-j");
            cmd.arg(jobs.to_string());
        }

        if let Some(targets) = &self.targets {
            for target in targets {
                cmd.arg(target);
            }
        }

        cmd.status()
    }
}

pub fn alias<'a>(input: &'a str, alias: &'a str) -> String {
    NinjaBuildBuilder::default()
        .rule("phony")
        .input(Cow::from(Utf8Path::new(input)))
        .out(Cow::from(Utf8Path::new(alias)))
        .build()
        .unwrap()
        .to_string()
}

pub fn alias_multiple<'a>(inputs: Vec<Cow<'a, Utf8Path>>, alias: &'a str) -> String {
    NinjaBuildBuilder::default()
        .rule("phony")
        .inputs(inputs)
        .out(Cow::from(Utf8Path::new(alias)))
        .build()
        .unwrap()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn basic() {
    //     let mut file = NinjaWriter::new(Utf8Path::new("ninja.build")).unwrap();
    //     file.file.write(b"foo\n").unwrap();
    // }

    #[test]
    fn rule() {
        let rule = NinjaRuleBuilder::default()
            .name("CC")
            .command("gcc ${CFLAGS} -c ${in} ${out}")
            .description(Cow::from("Compile"))
            .build()
            .unwrap();
        assert_eq!(
            concat!(
                "rule CC\n",
                "  command = gcc ${CFLAGS} -c ${in} ${out}\n",
                "  description = Compile\n",
                "\n"
            ),
            format!("{}", rule)
        );
    }

    #[test]
    fn build_simple() {
        let out = Utf8PathBuf::from("test.o");
        let rule = NinjaBuildBuilder::default()
            .rule("CC")
            .out(out.as_path())
            .build()
            .unwrap();
        assert_eq!(
            concat!("build test.o: $\n", "    CC\n", "\n"),
            format!("{}", rule)
        );
    }

    #[test]
    fn build_with_input() {
        let testc = Utf8PathBuf::from("test.c");
        let test2c = Utf8PathBuf::from("test2.c");
        let in_vec = vec![testc.as_path().into(), test2c.as_path().into()];
        let out = Utf8PathBuf::from("test.o");
        let rule = NinjaBuildBuilder::default()
            .rule("CC")
            .inputs(in_vec)
            .out(out.as_path())
            .build()
            .unwrap();

        assert_eq!(
            concat!(
                "build test.o: $\n",
                "    CC $\n",
                "    test.c $\n",
                "    test2.c\n",
                "\n"
            ),
            format!("{}", rule)
        );
    }

    #[test]
    fn build_with_deps() {
        let testc = Utf8PathBuf::from("test.c");
        let test2c = Utf8PathBuf::from("test2.c");
        let in_vec = vec![testc.as_path().into(), test2c.as_path().into()];
        let deps = vec![
            Utf8Path::new("other.o").into(),
            Utf8Path::new("other2.o").into(),
        ];
        let out = Utf8PathBuf::from("test.o");
        let rule = NinjaBuildBuilder::default()
            .rule("CC")
            .inputs(in_vec)
            .out(out.as_path())
            .deps(deps)
            .build()
            .unwrap();

        assert_eq!(
            concat!(
                "build test.o: $\n",
                "    CC $\n",
                "    test.c $\n",
                "    test2.c $\n",
                "    | $\n",
                "    other.o $\n",
                "    other2.o\n",
                "\n"
            ),
            format!("{}", rule)
        );
    }

    #[test]
    fn alias() {
        let build = super::alias(Utf8Path::new("foo").as_str(), "foo_alias");
        assert_eq!(build, "build foo_alias: $\n    phony $\n    foo\n\n");
    }
}
