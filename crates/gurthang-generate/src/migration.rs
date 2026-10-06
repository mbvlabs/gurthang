use std::{fs, io::Write, path::Path};

use gurthang_project::find_root;

use crate::{Error, GenerateOptions, region::write_if_allowed};

pub fn generate(name: &str, options: GenerateOptions, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let directory = root.join("migrations");
    fs::create_dir_all(&directory)?;
    let next = next_version(&directory)?;
    let slug = slug(name);
    let relative = format!("migrations/{next:04}_{slug}.sql");
    let path = root.join(&relative);
    let contents = format!("-- {slug}\n");
    if options.dry_run {
        writeln!(out, "Would write {relative}")?;
        return Ok(());
    }
    write_if_allowed(&path, &contents, false, false)?;
    writeln!(out, "Created {relative}")?;
    Ok(())
}

fn next_version(directory: &Path) -> Result<u32, Error> {
    let mut max = 0;
    if directory.is_dir() {
        for entry in fs::read_dir(directory)? {
            let name = entry?.file_name();
            let name = name.to_string_lossy();
            if let Some((version, _)) = name.split_once('_')
                && let Ok(parsed) = version.parse::<u32>()
            {
                max = max.max(parsed);
            }
        }
    }
    Ok(max + 1)
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
