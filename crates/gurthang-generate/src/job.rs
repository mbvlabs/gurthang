use std::{fs, io::Write};

use gurthang_project::find_root;
use heck::{ToPascalCase, ToSnakeCase};

use crate::{Error, GenerateOptions, region};

const VARIANTS_START: &str = "// gurthang:generated:variants:start";
const VARIANTS_END: &str = "// gurthang:generated:variants:end";
const NAMES_START: &str = "// gurthang:generated:names:start";
const NAMES_END: &str = "// gurthang:generated:names:end";
const HANDLERS_START: &str = "// gurthang:generated:handlers:start";
const HANDLERS_END: &str = "// gurthang:generated:handlers:end";

pub fn generate(name: &str, options: GenerateOptions, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let path = root.join("src/jobs/mod.rs");
    if !path.exists() {
        return Err(Error::Message("src/jobs/mod.rs is missing".into()));
    }
    let pascal = name.to_pascal_case();
    let snake = name.to_snake_case();
    let source = fs::read_to_string(&path)?;
    if source.contains(&format!("Self::{pascal}")) || source.contains(&format!("{pascal},")) {
        return Err(Error::Message(format!("job {pascal} already exists")));
    }
    if options.dry_run {
        writeln!(out, "Would add Job::{pascal}")?;
        return Ok(());
    }
    let source = region::ensure_line_in_region(
        &source,
        VARIANTS_START,
        VARIANTS_END,
        &format!("{pascal},"),
    )?;
    let source = region::ensure_line_in_region(
        &source,
        NAMES_START,
        NAMES_END,
        &format!("Self::{pascal} => \"{snake}\","),
    )?;
    let source = region::ensure_line_in_region(
        &source,
        HANDLERS_START,
        HANDLERS_END,
        &format!("Self::{pascal} => Ok(()),"),
    )?;
    fs::write(&path, source)?;
    writeln!(out, "Added Job::{pascal}")?;
    Ok(())
}
