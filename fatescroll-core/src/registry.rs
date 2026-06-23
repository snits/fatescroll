// ABOUTME: In-memory table store keyed by fully qualified ID.
// ABOUTME: Supports relative-first reference resolution for chain/compound lookups.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::error::ValidationError;
use crate::models::Table;

#[derive(Debug)]
pub struct Registry {
    tables: HashMap<String, Table>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn register(&mut self, fqid: String, table: Table) -> Result<(), ValidationError> {
        match self.tables.entry(fqid) {
            Entry::Occupied(e) => Err(ValidationError::DuplicateId {
                id: e.key().clone(),
            }),
            Entry::Vacant(e) => {
                e.insert(table);
                Ok(())
            }
        }
    }

    pub fn get(&self, fqid: &str) -> Option<&Table> {
        self.tables.get(fqid)
    }

    /// Resolve a reference using three-step resolution:
    /// 1. Try current_namespace + "." + reference (relative)
    /// 2. Try reference as a fully qualified ID (absolute)
    /// 3. Search all tables by bare id suffix (global fallback, unambiguous only)
    ///
    /// Returns (fqid, &Table) on success.
    pub fn resolve(&self, reference: &str, current_namespace: &str) -> Option<(&str, &Table)> {
        let relative_id = format!("{current_namespace}.{reference}");
        if let Some((key, table)) = self.tables.get_key_value(&relative_id) {
            return Some((key, table));
        }

        if let Some((key, table)) = self.tables.get_key_value(reference) {
            return Some((key, table));
        }

        // Step 3: Global bare id fallback — search all tables by bare id suffix
        let suffix = format!(".{reference}");
        let candidates: Vec<_> = self
            .tables
            .iter()
            .filter(|(fqid, _)| fqid.ends_with(&suffix))
            .collect();
        if candidates.len() == 1 {
            let (fqid, table) = candidates[0];
            return Some((fqid.as_str(), table));
        }

        None
    }

    pub fn all_tables(&self) -> impl Iterator<Item = (&str, &Table)> {
        self.tables.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};

    fn simple_table(name: &str) -> Table {
        Table::Simple {
            id: name.to_lowercase().replace(' ', "-"),
            name: name.to_string(),
            tags: vec!["test".to_string()],
            notes: vec![],
            roll: "1d6".to_string(),
            modifier_range: None,
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("X".into()),
                chain: None,
            }],
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = Registry::new();
        reg.register("test.foo".into(), simple_table("Foo"))
            .unwrap();
        assert!(reg.get("test.foo").is_some());
        assert!(reg.get("test.bar").is_none());
    }

    #[test]
    fn duplicate_registration_fails() {
        let mut reg = Registry::new();
        reg.register("test.foo".into(), simple_table("Foo"))
            .unwrap();
        let err = reg.register("test.foo".into(), simple_table("Foo2"));
        assert!(err.is_err());
    }

    #[test]
    fn resolve_relative_first() {
        let mut reg = Registry::new();
        reg.register("ns.sub.target".into(), simple_table("Local"))
            .unwrap();
        reg.register("target".into(), simple_table("Global"))
            .unwrap();

        // Relative resolution: "target" in namespace "ns.sub" finds "ns.sub.target"
        let (fqid, table) = reg.resolve("target", "ns.sub").unwrap();
        assert_eq!(fqid, "ns.sub.target");
        assert_eq!(table.name(), "Local");
    }

    #[test]
    fn resolve_falls_back_to_fqid() {
        let mut reg = Registry::new();
        reg.register("other.target".into(), simple_table("Other"))
            .unwrap();

        // No relative match, but "other.target" works as FQID
        let (fqid, table) = reg.resolve("other.target", "ns.sub").unwrap();
        assert_eq!(fqid, "other.target");
        assert_eq!(table.name(), "Other");
    }

    #[test]
    fn resolve_not_found() {
        let reg = Registry::new();
        assert!(reg.resolve("nonexistent", "ns").is_none());
    }

    #[test]
    fn resolve_cross_namespace_by_bare_id() {
        let mut reg = Registry::new();
        let encounter_table = Table::Simple {
            id: "wolf-count".into(),
            name: "Wolf Count".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![],
        };
        reg.register("test.encounters.wolf-count".into(), encounter_table)
            .unwrap();

        // Resolve from a different namespace using bare id
        let result = reg.resolve("wolf-count", "test.npc");
        assert!(result.is_some(), "bare id should resolve across namespaces");
        let (fqid, _) = result.unwrap();
        assert_eq!(fqid, "test.encounters.wolf-count");
    }

    #[test]
    fn resolve_ambiguous_bare_id_returns_none() {
        let mut reg = Registry::new();
        let table1 = Table::Simple {
            id: "wolf-count".into(),
            name: "Wolf Count 1".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![],
        };
        let table2 = Table::Simple {
            id: "wolf-count".into(),
            name: "Wolf Count 2".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![],
        };
        reg.register("test.encounters.wolf-count".into(), table1)
            .unwrap();
        reg.register("test.npc.wolf-count".into(), table2).unwrap();

        // Resolve from a third namespace — ambiguous, should return None
        let result = reg.resolve("wolf-count", "test.terrain");
        assert!(result.is_none(), "ambiguous bare id should return None");
    }

    #[test]
    fn resolve_relative_takes_priority_over_global() {
        let mut reg = Registry::new();
        let local = Table::Simple {
            id: "wolf-count".into(),
            name: "Local Wolf Count".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![],
        };
        let remote = Table::Simple {
            id: "wolf-count".into(),
            name: "Remote Wolf Count".into(),
            tags: vec![],
            notes: vec![],
            roll: "1d4".into(),
            modifier_range: None,
            results: vec![],
        };
        reg.register("test.encounters.wolf-count".into(), local)
            .unwrap();
        reg.register("test.npc.wolf-count".into(), remote).unwrap();

        // Resolve from test.encounters — should get the local one (relative match)
        let result = reg.resolve("wolf-count", "test.encounters");
        assert!(result.is_some());
        let (fqid, table) = result.unwrap();
        assert_eq!(fqid, "test.encounters.wolf-count");
        assert_eq!(table.name(), "Local Wolf Count");
    }

    #[test]
    fn all_tables_iterates_everything() {
        let mut reg = Registry::new();
        reg.register("a.one".into(), simple_table("One")).unwrap();
        reg.register("b.two".into(), simple_table("Two")).unwrap();
        assert_eq!(reg.all_tables().count(), 2);
    }
}
