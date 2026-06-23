// ABOUTME: Search functions for finding tables by name, tag, or namespace.
// ABOUTME: All searches are case-insensitive for names, exact-match for tags.

use std::collections::BTreeSet;

use crate::models::Table;
use crate::registry::Registry;

/// Sort search results by FQID ascending for stable, deterministic output.
fn sort_by_fqid<'a>(mut results: Vec<(&'a str, &'a Table)>) -> Vec<(&'a str, &'a Table)> {
    results.sort_by(|a, b| a.0.cmp(b.0));
    results
}

/// Search by table name (case-insensitive substring match).
pub fn search_by_name<'a>(registry: &'a Registry, query: &str) -> Vec<(&'a str, &'a Table)> {
    let query_lower = query.to_lowercase();
    sort_by_fqid(
        registry
            .all_tables()
            .filter(|(_, table)| table.name().to_lowercase().contains(&query_lower))
            .collect(),
    )
}

/// Search by tag (exact match).
pub fn search_by_tag<'a>(registry: &'a Registry, tag: &str) -> Vec<(&'a str, &'a Table)> {
    sort_by_fqid(
        registry
            .all_tables()
            .filter(|(_, table)| table.tags().iter().any(|t| t == tag))
            .collect(),
    )
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
    sort_by_fqid(
        registry
            .all_tables()
            .filter(|(fqid, _)| fqid.starts_with(&prefix))
            .collect(),
    )
}

/// Collect all unique tags across all tables, sorted alphabetically.
pub fn collect_tags(registry: &Registry) -> BTreeSet<&str> {
    registry
        .all_tables()
        .flat_map(|(_, table)| table.tags().iter().map(|t| t.as_str()))
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
                id: "gems".into(),
                name: "Gem Type".into(),
                tags: vec!["treasure".into(), "gems".into()],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
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
                id: "wilderness".into(),
                name: "Wilderness Encounter".into(),
                tags: vec!["encounter".into(), "wilderness".into()],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
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
                id: "occupation".into(),
                name: "NPC Occupation".into(),
                tags: vec!["npc".into()],
                notes: vec![],
                roll: "1d6".into(),
                modifier_range: None,
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

    #[test]
    fn collect_tags_empty_registry() {
        let reg = Registry::new();
        let tags = collect_tags(&reg);
        assert!(tags.is_empty());
    }

    #[test]
    fn collect_tags_returns_sorted_unique_tags() {
        let reg = build_search_registry();
        let tags = collect_tags(&reg);
        assert_eq!(
            tags.iter().copied().collect::<Vec<_>>(),
            vec!["encounter", "gems", "npc", "treasure", "wilderness"]
        );
    }

    fn make_simple_table(id: &str) -> Table {
        Table::Simple {
            id: id.into(),
            name: id.into(),
            tags: vec!["zzz-tag".into()],
            notes: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("result".into()),
                chain: None,
            }],
        }
    }

    #[test]
    fn search_results_are_sorted_by_fqid() {
        // Register 5 tables in a deliberately scrambled order to expose HashMap non-determinism.
        // The search functions must return them sorted by FQID regardless of insertion order.
        let mut reg = Registry::new();
        reg.register("zzz.echo".into(), make_simple_table("echo"))
            .unwrap();
        reg.register("zzz.alpha".into(), make_simple_table("alpha"))
            .unwrap();
        reg.register("zzz.delta".into(), make_simple_table("delta"))
            .unwrap();
        reg.register("zzz.bravo".into(), make_simple_table("bravo"))
            .unwrap();
        reg.register("zzz.charlie".into(), make_simple_table("charlie"))
            .unwrap();

        let expected = vec![
            "zzz.alpha",
            "zzz.bravo",
            "zzz.charlie",
            "zzz.delta",
            "zzz.echo",
        ];

        let by_ns: Vec<&str> = search_by_namespace(&reg, "zzz")
            .iter()
            .map(|(fqid, _)| *fqid)
            .collect();
        assert_eq!(
            by_ns, expected,
            "search_by_namespace must be sorted by FQID"
        );

        let by_name: Vec<&str> = search_by_name(&reg, "")
            .iter()
            .map(|(fqid, _)| *fqid)
            .collect();
        // search_by_name with empty query matches all tables, but we only have zzz.* here
        // so the filtered set is the same 5; order must be sorted.
        // NOTE: build_search_registry tables have no "zzz" namespace, so this reg is isolated.
        assert_eq!(by_name, expected, "search_by_name must be sorted by FQID");

        let by_tag: Vec<&str> = search_by_tag(&reg, "zzz-tag")
            .iter()
            .map(|(fqid, _)| *fqid)
            .collect();
        assert_eq!(by_tag, expected, "search_by_tag must be sorted by FQID");
    }
}
