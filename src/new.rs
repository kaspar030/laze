use std::borrow::Cow;

use anyhow::{bail, Context as _, Error, Result};
use camino::Utf8PathBuf;
use clap::ArgMatches;
use rust_embed::RustEmbed;
use serde::Serialize;

use tinytemplate::TinyTemplate;

#[derive(Serialize, RustEmbed)]
#[folder = "assets/templates"]
struct TemplateFiles;

#[derive(Serialize)]
struct Context {
    project_name: String,
}

trait PathEmpty {
    fn is_empty(&self) -> Result<bool, Error>;
}

impl PathEmpty for camino::Utf8Path {
    fn is_empty(&self) -> Result<bool, Error> {
        Ok(self.exists() && self.is_file()
            || (self.is_dir() && { self.read_dir()?.next().is_none() }))
    }
}

pub fn from_matches(matches: &ArgMatches) -> Result<(), Error> {
    let path = matches.get_one::<Utf8PathBuf>("path").unwrap();
    let template_name = matches.get_one::<String>("template").unwrap();
    let _verbose = matches.get_count("verbose");
    let quiet = matches.get_count("quiet");

    let prefix = format!("{template_name}/");
    if !TemplateFiles::iter().any(|x| x.starts_with(&prefix)) {
        bail!("no internal template with name \"{template_name}\" available");
    }

    if path.exists() {
        if !path.is_empty()? {
            bail!("path \"{path}\" exists and is not empty");
        }
    } else {
        std::fs::create_dir_all(path).with_context(|| format!("creating {path}"))?;
    }

    let path = path
        .canonicalize_utf8()
        .with_context(|| format!("canonicalizing {path}"))?;

    let context = Context {
        project_name: path.file_name().unwrap().to_string(),
    };

    for filename in TemplateFiles::iter().filter(|x| x.starts_with(&prefix)) {
        let embedded_file = TemplateFiles::get(&filename).unwrap();
        let in_filename = filename.strip_prefix(&prefix).unwrap();
        let filename = path.join(in_filename);

        let directory = filename.parent().unwrap();
        std::fs::create_dir_all(directory).with_context(|| format!("creating {directory}"))?;

        let file_data;
        if filename.extension().eq(&Some("in")) {
            let mut outfile = filename.clone();
            outfile.set_file_name(filename.clone().file_stem().unwrap());

            let mut tt = TinyTemplate::new();
            let template = std::str::from_utf8(&embedded_file.data)?;
            tt.add_template("", template)
                .with_context(|| format!("parsing \"{in_filename}\""))?;

            let rendered = tt
                .render("", &context)
                .with_context(|| format!("rendering \"{in_filename}\""))?;

            file_data = Cow::from(rendered.as_bytes());
            std::fs::write(&outfile, file_data).with_context(|| format!("creating {outfile}"))?;
        } else {
            std::fs::write(&filename, embedded_file.data)
                .with_context(|| format!("creating {filename}"))?;
        };
    }

    if quiet == 0 {
        println!("laze: created '{}' project", context.project_name);
    }

    Ok(())
}
