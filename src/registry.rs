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
            Entry::Occupied(e) => Err(ValidationError::DuplicateId { id: e.key().clone() }),
            Entry::Vacant(e) => {
                e.insert(table);
                Ok(())
            }
        }
    }

    pub fn get(&self, fqid: &str) -> Option<&Table> {
        self.tables.get(fqid)
    }

    /// Resolve a reference using relative-first resolution:
    /// 1. Try current_namespace + "." + reference
    /// 2. Try reference as a fully qualified ID
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
            name: name.to_string(),
            tags: vec!["test".to_string()],
            roll: "1d6".to_string(),
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
        reg.register("test.foo".into(), simple_table("Foo")).unwrap();
        assert!(reg.get("test.foo").is_some());
        assert!(reg.get("test.bar").is_none());
    }

    #[test]
    fn duplicate_registration_fails() {
        let mut reg = Registry::new();
        reg.register("test.foo".into(), simple_table("Foo")).unwrap();
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
    fn all_tables_iterates_everything() {
        let mut reg = Registry::new();
        reg.register("a.one".into(), simple_table("One")).unwrap();
        reg.register("b.two".into(), simple_table("Two")).unwrap();
        assert_eq!(reg.all_tables().count(), 2);
    }
}
