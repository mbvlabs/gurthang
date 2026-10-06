use crate::error::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectName {
    project_name: String,
    crate_name: String,
    package_name: String,
}

impl ProjectName {
    pub fn parse(value: &str) -> Result<Self> {
        if value.is_empty() {
            return Err(Error::InvalidProjectName("name cannot be empty".into()));
        }
        if value == "." || value == ".." || value.contains('/') || value.contains('\\') {
            return Err(Error::InvalidProjectName(
                "name must not be a path or contain path separators".into(),
            ));
        }
        if !value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(Error::InvalidProjectName(
                "name must start with an ASCII letter or digit".into(),
            ));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(Error::InvalidProjectName(
                "use only ASCII letters, digits, hyphens, and underscores".into(),
            ));
        }

        let lowercase = value.to_ascii_lowercase();
        let mut crate_name = lowercase.replace('-', "_");
        if crate_name.as_bytes()[0].is_ascii_digit() || is_rust_keyword(&crate_name) {
            crate_name.insert(0, '_');
        }
        let package_name = lowercase.replace('_', "-");

        Ok(Self {
            project_name: value.to_owned(),
            crate_name,
            package_name,
        })
    }

    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_identifiers_from_hyphenated_name() {
        let name = ProjectName::parse("My-App").unwrap();
        assert_eq!(name.project_name(), "My-App");
        assert_eq!(name.crate_name(), "my_app");
        assert_eq!(name.package_name(), "my-app");
    }

    #[test]
    fn prefixes_crate_identifiers_that_start_with_a_digit() {
        assert_eq!(ProjectName::parse("2fast").unwrap().crate_name(), "_2fast");
        assert_eq!(ProjectName::parse("crate").unwrap().crate_name(), "_crate");
    }

    #[test]
    fn rejects_paths_and_invalid_characters() {
        for invalid in [
            "",
            ".",
            "..",
            "../demo",
            "a/b",
            "a\\b",
            "-demo",
            "hello world",
        ] {
            assert!(ProjectName::parse(invalid).is_err(), "accepted {invalid:?}");
        }
    }
}
