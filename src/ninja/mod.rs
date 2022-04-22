use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use indexmap::IndexMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NinjaRuleDeps {
    None,
    GCC(String),
}

impl Default for NinjaRuleDeps {
    fn default() -> NinjaRuleDeps {
        NinjaRuleDeps::None
    }
}

#[derive(Default, Builder, Debug, PartialEq, Eq, Clone)]
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
        write!(
            f,
            "rule {}\n  command = {}\n{}{}{}{}{}\n",
            self.name,
            self.command,
            match &self.description {
                Some(description) => format!("  description = {}\n", description),
                None => String::new(),
            },
            match &self.deps {
                NinjaRuleDeps::None => String::new(),
                NinjaRuleDeps::GCC(s) => format!("  deps = gcc\n  depfile = {}\n", s),
            },
            match &self.rspfile {
                Some(rspfile) => format!("  rspfile = {}\n", rspfile),
                None => String::new(),
            },
            match &self.rspfile_content {
                Some(rspfile_content) => format!("  rspfile_content = {}\n", rspfile_content),
                None => String::new(),
            },
            match &self.pool {
                Some(pool) => format!("  pool = {}\n", pool),
                None => String::new(),
            },
        )
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
    inputs: Option<Vec<Cow<'a, Path>>>,
    outs: Vec<Cow<'a, Path>>,

    #[builder(default = "None")]
    deps: Option<Vec<Cow<'a, Path>>>,

    #[builder(setter(into, strip_option), default = "None")]
    env: Option<&'a IndexMap<String, String>>,

    #[builder(default = "false")]
    always: bool,
    //   deps: NinjaRuleDeps,
}

impl<'a> NinjaBuildBuilder<'a> {
    pub fn out<I>(&mut self, out: I) -> &mut Self
    where
        I: Into<Cow<'a, Path>>,
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
        I: Into<Cow<'a, Path>>,
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
}

impl<'a> fmt::Display for NinjaBuild<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "build")?;

        for out in &self.outs {
            write!(f, " {}", out.to_str().unwrap())?;
        }

        write!(f, ": $\n    {}", self.rule)?;

        if let Some(inputs) = &self.inputs {
            for path in inputs {
                write!(f, " $\n    {}", path.to_str().unwrap())?;
            }
        }

        if self.deps.is_some() || self.always {
            write!(f, " $\n    |")?;
            if let Some(list) = &self.deps {
                for entry in list {
                    write!(f, " $\n    {}", entry.to_str().unwrap())?;
                }
            }
            if self.always {
                write!(f, " $\n    ALWAYS")?;
            }
        }
        writeln!(f)?;

        if let Some(env) = self.env {
            for (k, v) in env {
                writeln!(f, "  {} = {}", k, v)?;
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
//     pub fn new(path: &Path) -> std::io::Result<NinjaWriter> {
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

#[derive(Default, Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct NinjaCmd<'a> {
    #[builder(setter(into), default = "\"ninja\"")]
    binary: &'a str,

    #[builder(setter(into), default = "\"build.ninja\"")]
    build_file: &'a str,

    #[builder(default = "false")]
    verbose: bool,

    #[builder(default = "None")]
    targets: Option<Vec<PathBuf>>,

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

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn basic() {
    //     let mut file = NinjaWriter::new(Path::new("ninja.build")).unwrap();
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
        let out = PathBuf::from("test.o");
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
        let testc = PathBuf::from("test.c");
        let test2c = PathBuf::from("test2.c");
        let in_vec = vec![testc.as_path().into(), test2c.as_path().into()];
        let out = PathBuf::from("test.o");
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
        let testc = PathBuf::from("test.c");
        let test2c = PathBuf::from("test2.c");
        let in_vec = vec![testc.as_path().into(), test2c.as_path().into()];
        let deps = vec![Path::new("other.o").into(), Path::new("other2.o").into()];
        let out = PathBuf::from("test.o");
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
}
