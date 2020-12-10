use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

pub struct NinjaWriter {
    pub file: BufWriter<File>,
    pub rules: HashSet<u64>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NinjaRuleDeps {
    None,
    GCC(String),
}

impl Default for NinjaRuleDeps {
    fn default() -> NinjaRuleDeps {
        return NinjaRuleDeps::None;
    }
}

#[derive(Default, Builder, Debug, PartialEq, Eq, Clone)]
//#[builder(setter(into))]
pub struct NinjaRule<'a> {
    pub name: Cow<'a, str>,
    command: Cow<'a, str>,
    description: Option<Cow<'a, str>>,
    #[builder(setter(into, strip_option), default = "None")]
    env: Option<&'a HashMap<String, String>>,
    #[builder(default = "NinjaRuleDeps::None")]
    deps: NinjaRuleDeps,
    #[builder(default = "None")]
    rspfile: Option<Cow<'a, str>>,
    #[builder(default = "None")]
    rspfile_content: Option<Cow<'a, str>>,
}

impl<'a> fmt::Display for NinjaRule<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "rule {}\n  command = {}\n{}{}{}{}\n",
            self.name,
            self.command,
            match &self.description {
                Some(description) => format!("  description = {}\n", description),
                None => format!(""),
            },
            match &self.deps {
                NinjaRuleDeps::None => format!(""),
                NinjaRuleDeps::GCC(s) => format!("  deps = gcc\n  depfile = {}\n", s),
            },
            match &self.rspfile {
                Some(rspfile) => format!("  rspfile = {}\n", rspfile),
                None => format!(""),
            },
            match &self.rspfile_content {
                Some(rspfile_content) => format!("  rspfile_content = {}\n", rspfile_content),
                None => format!(""),
            },
        )
    }
}

impl<'a> NinjaRule<'a> {
    pub fn get_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    pub fn get_hashed_name(&self, hash: u64) -> String {
        let mut name = String::from(self.name.clone());
        name.push_str(&format!("_{}", hash));
        name
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
    out: Cow<'a, Path>,

    #[builder(setter(strip_option), default = "None")]
    in_vec: Option<Vec<Cow<'a, Path>>>,
    #[builder(setter(strip_option), default = "None")]
    in_single: Option<Cow<'a, Path>>,

    #[builder(setter(into, strip_option), default = "None")]
    deps: Option<Vec<Cow<'a, Path>>>,

    #[builder(setter(into, strip_option), default = "None")]
    env: Option<&'a HashMap<String, String>>,
    //   deps: NinjaRuleDeps,
}

impl<'a> fmt::Display for NinjaBuild<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "build {}: $\n    {}",
            self.out.to_str().unwrap(),
            self.rule
        )?;

        if let Some(list) = &self.in_vec {
            for path in list {
                write!(f, " $\n    {}", path.to_str().unwrap())?;
            }
        }

        if let Some(path) = &self.in_single {
            write!(f, " $\n    {}", path.to_str().unwrap())?;
        }

        if let Some(list) = &self.deps {
            write!(f, " $\n    | $\n    ")?;
            for entry in list {
                write!(f, "{} $\n    ", entry.to_str().unwrap())?;
            }
            write!(f, "\n")?;
        } else {
            write!(f, "\n")?;
        }

        if let Some(env) = self.env {
            for (k, v) in env {
                write!(f, "  {} = {}\n", k, v)?;
            }
        }

        write!(f, "\n")
    }
}

impl NinjaWriter {
    pub fn new(path: &Path) -> std::io::Result<NinjaWriter> {
        Ok(NinjaWriter {
            file: BufWriter::new(File::create(path)?),
            rules: HashSet::new(),
        })
    }

    pub fn write_rule(&mut self, rule: &NinjaRule) -> std::io::Result<()> {
        self.file.write_all(format!("{}", rule).as_bytes())
    }

    pub fn write_rule_dedup(&mut self, rule: &NinjaRule) -> std::io::Result<String> {
        let rule_hash = rule.get_hash();
        let name = rule.get_hashed_name(rule_hash);

        if self.rules.insert(rule_hash) {
            let mut named = rule.clone();
            named.name = Cow::from(&name);
            self.write_rule(&named)?;
        }

        Ok(name)
    }

    pub fn write_var(&mut self, var: &str, val: &str) -> std::io::Result<()> {
        self.file
            .write_all(format!("{} = {}\n", var, val).as_bytes())
    }

    pub fn write_build(&mut self, build: &NinjaBuild) -> std::io::Result<()> {
        self.file.write_all(format!("{}", build).as_bytes())
    }
}

#[derive(Default, Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct NinjaCmd<'a> {
    #[builder(setter(into), default = "\"ninja\"")]
    binary: &'a str,

    #[builder(setter(into), default = "\"build.ninja\"")]
    build_file: &'a str,

    #[builder(default = "false")]
    verbose: bool,
}

impl<'a> NinjaCmd<'a> {
    pub fn run(&self) -> std::io::Result<ExitStatus> {
        let mut cmd = Command::new(self.binary);
        cmd.arg("-f").arg(self.build_file);

        if self.verbose {
            cmd.arg("-v");
        }

        cmd.status()
    }
}

#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut file = NinjaWriter::new(Path::new("ninja.build")).unwrap();
        file.file.write(b"foo\n");
    }

    #[test]
    fn rule() {
        let rule = NinjaRuleBuilder::default()
            .name("CC")
            .command("gcc ${CFLAGS} -c ${in} ${out}")
            .description("Compile")
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
        let in_vec = vec![testc.as_path(), test2c.as_path()];
        let out = PathBuf::from("test.o");
        let rule = NinjaBuildBuilder::default()
            .rule("CC")
            .in_vec(in_vec)
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
        let in_vec = vec![testc.as_path(), test2c.as_path()];
        let out = PathBuf::from("test.o");
        let rule = NinjaBuildBuilder::default()
            .rule("CC")
            .in_vec(in_vec)
            .out(out.as_path())
            .deps(vec!["other.o", "other2.o"])
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
