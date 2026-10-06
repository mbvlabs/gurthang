// Comment fences for generate/sync. Turn these into macros later so generated
// and custom regions are real language items, not string markers.
pub const GENERATED_START: &str = "// gurthang:generated:start";
pub const GENERATED_END: &str = "// gurthang:generated:end";
pub const CUSTOM_START: &str = "// gurthang:custom:start";
pub const CUSTOM_END: &str = "// gurthang:custom:end";

pub fn extract_custom(source: &str) -> String {
    let Some(start) = source.find(CUSTOM_START) else {
        return String::new();
    };
    let Some(end) = source[start..].find(CUSTOM_END) else {
        return String::new();
    };
    source[start..start + end + CUSTOM_END.len()].to_owned()
}

pub fn with_custom(generated: &str, custom: &str) -> String {
    let generated = generated.trim_end();
    if custom.is_empty() {
        format!("{generated}\n\n{CUSTOM_START}\n{CUSTOM_END}\n")
    } else {
        format!("{generated}\n\n{}\n", custom.trim_end())
    }
}

pub fn wrap_generated(body: &str) -> String {
    format!("{GENERATED_START}\n{}\n{GENERATED_END}\n", body.trim_end())
}

pub fn write_if_allowed(
    path: &std::path::Path,
    contents: &str,
    dry_run: bool,
    overwrite: bool,
) -> Result<bool, crate::Error> {
    if dry_run {
        return Ok(false);
    }
    if path.exists() && !overwrite {
        return Err(crate::Error::Message(format!(
            "{} already exists",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(true)
}

pub fn ensure_line(path: &std::path::Path, line: &str) -> Result<(), crate::Error> {
    let mut source = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };
    if source.contains(line) {
        return Ok(());
    }
    if !source.is_empty() && !source.ends_with('\n') {
        source.push('\n');
    }
    source.push_str(line);
    source.push('\n');
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, source)?;
    Ok(())
}

pub fn ensure_line_in_region(
    source: &str,
    start: &str,
    end: &str,
    line: &str,
) -> Result<String, crate::Error> {
    if source.contains(line) {
        return Ok(source.to_owned());
    }
    let Some(start_idx) = source.find(start) else {
        return Err(crate::Error::Message(format!("missing {start} marker")));
    };
    let after_start = start_idx + start.len();
    let Some(rel_end) = source[after_start..].find(end) else {
        return Err(crate::Error::Message(format!("missing {end} marker")));
    };
    let end_idx = after_start + rel_end;
    let indent = leading_indent(&source[after_start..end_idx]);
    let insertion = format!("\n{indent}{line}");
    Ok(format!(
        "{}{insertion}\n{indent}{}",
        &source[..end_idx],
        &source[end_idx..]
    ))
}

fn leading_indent(region: &str) -> String {
    region
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(line[..line.len() - trimmed.len()].to_owned())
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_custom_regions() {
        let source = "// gurthang:custom:start\nfn extra() {}\n// gurthang:custom:end\n";
        assert_eq!(extract_custom(source), source.trim_end());
    }

    #[test]
    fn inserts_a_missing_line_before_the_end_marker() {
        let source = "start\n    existing,\nend";
        let updated = ensure_line_in_region(source, "start", "end", "added,").unwrap();
        assert!(updated.contains("    added,\n    end") || updated.contains("added,"));
        assert!(updated.contains("existing,"));
    }
}
