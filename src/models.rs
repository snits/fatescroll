// ABOUTME: Data models for tables, manifests, and roll results.
// ABOUTME: Serde structs for YAML deserialization and RollResult output type.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct ResultEntry {
    pub min: u32,
    pub max: u32,
    pub text: Option<String>,
    pub chain: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Table {
    #[serde(rename = "simple")]
    Simple {
        id: String,
        name: String,
        #[serde(default)]
        tags: Vec<String>,
        roll: String,
        results: Vec<ResultEntry>,
    },
    #[serde(rename = "compound")]
    Compound {
        id: String,
        name: String,
        #[serde(default)]
        tags: Vec<String>,
        tables: Vec<String>,
    },
}

impl Table {
    pub fn id(&self) -> &str {
        match self {
            Table::Simple { id, .. } | Table::Compound { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Table::Simple { name, .. } | Table::Compound { name, .. } => name,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            Table::Simple { tags, .. } | Table::Compound { tags, .. } => tags,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub namespace: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub author: Option<String>,
    pub min_tool_version: Option<String>,
    pub directories: Vec<DirectoryEntry>,
    #[serde(skip)]
    pub base_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RollResult {
    pub table_name: String,
    pub roll: Option<u32>,
    pub text: Option<String>,
    pub children: Vec<RollResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_simple_table() {
        let yaml = r#"
name: Test Table
type: simple
tags:
  - test
roll: 1d6
results:
  - min: 1
    max: 3
    text: Low
  - min: 4
    max: 6
    text: High
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple {
                name,
                tags,
                roll,
                results,
            } => {
                assert_eq!(name, "Test Table");
                assert_eq!(tags, vec!["test"]);
                assert_eq!(roll, "1d6");
                assert_eq!(results.len(), 2);
                assert_eq!(results[0].min, 1);
                assert_eq!(results[0].max, 3);
                assert_eq!(results[0].text.as_deref(), Some("Low"));
                assert!(results[0].chain.is_none());
            }
            _ => panic!("Expected Simple table"),
        }
    }

    #[test]
    fn deserialize_simple_table_with_chains() {
        let yaml = r#"
name: Encounter
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 2
    text: Wolves
    chain:
      - wolf-count
  - min: 3
    max: 4
    text: Bandits
    chain:
      - bandit-strength
      - bandit-motivation
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple { results, .. } => {
                assert_eq!(results[0].chain.as_ref().unwrap(), &["wolf-count"]);
                assert_eq!(
                    results[1].chain.as_ref().unwrap(),
                    &["bandit-strength", "bandit-motivation"]
                );
            }
            _ => panic!("Expected Simple table"),
        }
    }

    #[test]
    fn deserialize_compound_table() {
        let yaml = r#"
name: Quick NPC
type: compound
tags:
  - npc
  - generator
tables:
  - npc-occupation
  - npc-disposition
  - npc-quirk
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Compound { name, tags, tables } => {
                assert_eq!(name, "Quick NPC");
                assert_eq!(tags, vec!["npc", "generator"]);
                assert_eq!(
                    tables,
                    vec!["npc-occupation", "npc-disposition", "npc-quirk"]
                );
            }
            _ => panic!("Expected Compound table"),
        }
    }

    #[test]
    fn deserialize_manifest() {
        let yaml = r#"
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
  - path: encounters
    namespace: test.encounters
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "Test Collection");
        assert_eq!(manifest.version, "1.0");
        assert_eq!(manifest.namespace, "test");
        assert!(manifest.author.is_none());
        assert_eq!(manifest.directories.len(), 2);
        assert_eq!(manifest.directories[0].namespace, "test.terrain");
    }

    #[test]
    fn deserialize_simple_table_default_tags() {
        let yaml = r#"
name: Minimal
type: simple
roll: 1d4
results:
  - min: 1
    max: 4
    text: Something
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple { tags, .. } => assert!(tags.is_empty()),
            _ => panic!("Expected Simple table"),
        }
    }
}
