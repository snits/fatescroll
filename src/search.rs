// ABOUTME: Search functions for finding tables by name, tag, or namespace.
// ABOUTME: All searches are case-insensitive for names, exact-match for tags.

use crate::models::Table;
use crate::registry::Registry;

/// Search by table name (case-insensitive substring match).
pub fn search_by_name<'a>(registry: &'a Registry, query: &str) -> Vec<(&'a str, &'a Table)> {
    let query_lower = query.to_lowercase();
    registry
        .all_tables()
        .filter(|(_, table)| table.name().to_lowercase().contains(&query_lower))
        .collect()
}

/// Search by tag (exact match).
pub fn search_by_tag<'a>(registry: &'a Registry, tag: &str) -> Vec<(&'a str, &'a Table)> {
    registry
        .all_tables()
        .filter(|(_, table)| table.tags().iter().any(|t| t == tag))
        .collect()
}

/// Search by namespace prefix (FQID starts with the given namespace).
pub fn search_by_namespace<'a>(
    registry: &'a Registry,
    namespace: &str,
) -> Vec<(&'a str, &'a Table)> {
    let prefix = if namespace.ends_with('.') {
        namespace.to_string()
    } else {
        format!("{namespace}.")
    };
    registry
        .all_tables()
        .filter(|(fqid, _)| fqid.starts_with(&prefix))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};
    use crate::registry::Registry;

    fn build_search_registry() -> Registry {
        let mut reg = Registry::new();
        reg.register(
            "dmg.treasure.gems".into(),
            Table::Simple {
                name: "Gem Type".into(),
                tags: vec!["treasure".into(), "gems".into()],
                roll: "1d6".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Ruby".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();
        reg.register(
            "dmg.encounters.wilderness".into(),
            Table::Simple {
                name: "Wilderness Encounter".into(),
                tags: vec!["encounter".into(), "wilderness".into()],
                roll: "1d6".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Wolves".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();
        reg.register(
            "core.npc.occupation".into(),
            Table::Simple {
                name: "NPC Occupation".into(),
                tags: vec!["npc".into()],
                roll: "1d6".into(),
                results: vec![ResultEntry {
                    min: 1,
                    max: 6,
                    text: Some("Smith".into()),
                    chain: None,
                }],
            },
        )
        .unwrap();
        reg
    }

    #[test]
    fn search_by_name_substring() {
        let reg = build_search_registry();
        let results = search_by_name(&reg, "wilderness");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "dmg.encounters.wilderness");
    }

    #[test]
    fn search_by_name_case_insensitive() {
        let reg = build_search_registry();
        let results = search_by_name(&reg, "GEM");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_by_tag_exact_match() {
        let reg = build_search_registry();
        let results = search_by_tag(&reg, "npc");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "core.npc.occupation");
    }

    #[test]
    fn search_by_namespace_prefix() {
        let reg = build_search_registry();
        let results = search_by_namespace(&reg, "dmg");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_by_tag_is_case_sensitive() {
        let reg = build_search_registry();
        assert!(search_by_tag(&reg, "NPC").is_empty());
    }

    #[test]
    fn search_by_namespace_with_trailing_dot() {
        let reg = build_search_registry();
        let results = search_by_namespace(&reg, "dmg.");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_no_results() {
        let reg = build_search_registry();
        assert!(search_by_name(&reg, "nonexistent").is_empty());
        assert!(search_by_tag(&reg, "nonexistent").is_empty());
        assert!(search_by_namespace(&reg, "nonexistent").is_empty());
    }
}
