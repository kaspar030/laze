use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
#[builder(setter(into))]
pub struct NinjaRule<'a> {
    pub name: &'a str,
    command: String,
    description: Option<&'a str>,
    #[builder(setter(into, strip_option), default = "None")]
    env: Option<&'a HashMap<String, String>>,
    #[builder(default = "NinjaRuleDeps::None")]
    deps: NinjaRuleDeps,
}

impl<'a> fmt::Display for NinjaRule<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "rule {}\n  command = {}\n{}{}\n",
            self.name,
            self.command,
            match self.description {
                Some(description) => format!("  description = {}\n", description),
                None => format!(""),
            },
            match &self.deps {
                NinjaRuleDeps::None => format!(""),
                NinjaRuleDeps::GCC(s) => format!("  deps = gcc\n  depfile = {}\n", s),
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
}

impl<'a> Hash for NinjaRule<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.command.hash(state);
        self.description.hash(state);
    }
}

#[derive(Builder, Debug)]
#[builder(setter(into))]
pub struct NinjaBuild<'a> {
    rule: &'a str,
    out: &'a Path,

    #[builder(setter(strip_option), default = "None")]
    in_vec: Option<Vec<&'a Path>>,
    #[builder(setter(strip_option), default = "None")]
    in_single: Option<&'a Path>,

    #[builder(setter(into, strip_option), default = "None")]
    deps: Option<Vec<&'a str>>,

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
            write!(f, " $\n    | $\n    {}\n", list.join(" $\n    "))?;
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
        let mut name = String::from(rule.name);
        name.push_str(&format!("_{}", rule_hash));

        if self.rules.insert(rule_hash) {
            let mut named = rule.clone();
            named.name = &name[..];
            self.write_rule(&named)?;
        }

        Ok(name)
    }

    pub fn write_build(&mut self, build: &NinjaBuild) -> std::io::Result<()> {
        self.file.write_all(format!("{}", build).as_bytes())
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
