use std::collections::HashSet;

use anyhow::{anyhow, Result};

use super::Registry;

pub fn resolve_install_order(registry: &Registry, requested: &[String]) -> Result<Vec<String>> {
    let mut ordered = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for id in requested {
        visit(id, registry, &mut visiting, &mut visited, &mut ordered)?;
    }

    Ok(ordered)
}

fn visit(
    id: &str,
    registry: &Registry,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(id) {
        return Ok(());
    }

    if !visiting.insert(id.to_owned()) {
        return Err(anyhow!("dependency cycle detected at '{id}'"));
    }

    let tool = registry
        .get(id)
        .ok_or_else(|| anyhow!("unknown tool dependency '{id}'"))?;

    for dependency in &tool.definition.depends {
        visit(dependency, registry, visiting, visited, ordered)?;
    }

    visiting.remove(id);
    if visited.insert(id.to_owned()) {
        ordered.push(id.to_owned());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use include_dir::Dir;

    use crate::tools::{EmbeddedTool, Registry, Tool};

    use super::resolve_install_order;

    static EMPTY_DIR: Dir<'_> = Dir::new(".", &[]);

    fn registry(entries: &[(&str, &[&str])]) -> Registry {
        let tools = entries
            .iter()
            .map(|(id, depends)| {
                (
                    (*id).to_owned(),
                    EmbeddedTool {
                        definition: Tool {
                            id: (*id).to_owned(),
                            name: (*id).to_owned(),
                            description: String::new(),
                            version: "1".to_owned(),
                            depends: depends.iter().map(|dep| (*dep).to_owned()).collect(),
                            status_check: "true".to_owned(),
                        },
                        dir: &EMPTY_DIR,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        Registry { tools }
    }

    #[test]
    fn sorts_dependencies_before_dependents() {
        let registry = registry(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);
        let order = resolve_install_order(&registry, &["c".to_owned()]).unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn detects_cycles() {
        let registry = registry(&[("a", &["b"]), ("b", &["a"])]);
        let err = resolve_install_order(&registry, &["a".to_owned()]).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn deduplicates_shared_dependencies() {
        let registry = registry(&[("a", &[]), ("b", &["a"]), ("c", &["a"])]);
        let order = resolve_install_order(&registry, &["b".to_owned(), "c".to_owned()]).unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }
}
