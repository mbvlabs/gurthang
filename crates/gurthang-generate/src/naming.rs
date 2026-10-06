use heck::{ToPascalCase, ToSnakeCase};

#[derive(Clone, Debug)]
pub struct Resource {
    pub pascal: String,
    pub snake: String,
    pub plural_snake: String,
    pub plural_pascal: String,
    pub table: String,
    pub path: String,
}

impl Resource {
    pub fn parse(name: &str, table: Option<&str>) -> Self {
        let pascal = name.to_pascal_case();
        let snake = name.to_snake_case();
        let table = table
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| pluralize(&snake));
        let plural_snake = table.clone();
        let plural_pascal = table.to_pascal_case();
        let path = format!("/{table}");
        Self {
            pascal,
            snake,
            plural_snake,
            plural_pascal,
            table,
            path,
        }
    }
}

pub fn pluralize(name: &str) -> String {
    if name.ends_with('s') {
        name.to_owned()
    } else if name.ends_with('y') && name.len() > 1 {
        let mut plural = name[..name.len() - 1].to_owned();
        plural.push_str("ies");
        plural
    } else {
        format!("{name}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pluralizes_common_names() {
        assert_eq!(pluralize("post"), "posts");
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("users"), "users");
    }

    #[test]
    fn parses_table_overrides() {
        let resource = Resource::parse("Person", Some("people"));
        assert_eq!(resource.pascal, "Person");
        assert_eq!(resource.snake, "person");
        assert_eq!(resource.table, "people");
        assert_eq!(resource.plural_pascal, "People");
        assert_eq!(resource.path, "/people");
    }
}
