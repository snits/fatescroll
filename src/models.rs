// ABOUTME: Data models for tables, manifests, and roll results.
// ABOUTME: Serde structs for YAML deserialization and RollResult output type.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ChainRef {
    Simple(String),
    Modified {
        table: String,
        #[serde(default)]
        reroll: Vec<u32>,
    },
}

impl std::fmt::Display for ChainRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainRef::Simple(id) => write!(f, "{id}"),
            ChainRef::Modified { table, reroll } if reroll.is_empty() => {
                write!(f, "{table}")
            }
            ChainRef::Modified { table, reroll } => {
                write!(f, "{table} (reroll {:?})", reroll)
            }
        }
    }
}

impl ChainRef {
    pub fn table_id(&self) -> &str {
        match self {
            ChainRef::Simple(id) => id,
            ChainRef::Modified { table, .. } => table,
        }
    }

    pub fn reroll_values(&self) -> &[u32] {
        match self {
            ChainRef::Simple(_) => &[],
            ChainRef::Modified { reroll, .. } => reroll,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResultEntry {
    pub min: u32,
    pub max: u32,
    pub text: Option<String>,
    pub chain: Option<Vec<ChainRef>>,
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
pub struct FileEntry {
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
    #[serde(default)]
    pub directories: Vec<DirectoryEntry>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
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
id: test-table
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
                id,
                name,
                tags,
                roll,
                results,
            } => {
                assert_eq!(id, "test-table");
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
id: encounter
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
                let chains0 = results[0].chain.as_ref().unwrap();
                assert_eq!(chains0.len(), 1);
                assert_eq!(chains0[0].table_id(), "wolf-count");
                let chains1 = results[1].chain.as_ref().unwrap();
                assert_eq!(chains1.len(), 2);
                assert_eq!(chains1[0].table_id(), "bandit-strength");
                assert_eq!(chains1[1].table_id(), "bandit-motivation");
            }
            _ => panic!("Expected Simple table"),
        }
    }

    #[test]
    fn deserialize_compound_table() {
        let yaml = r#"
id: quick-npc
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
            Table::Compound {
                id,
                name,
                tags,
                tables,
            } => {
                assert_eq!(id, "quick-npc");
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
    fn deserialize_chain_ref_simple_string() {
        let yaml = r#""animal-type""#;
        let chain_ref: ChainRef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(chain_ref.table_id(), "animal-type");
        assert!(chain_ref.reroll_values().is_empty());
    }

    #[test]
    fn deserialize_chain_ref_with_reroll() {
        let yaml = r#"
table: wizard-mishap
reroll: [1]
"#;
        let chain_ref: ChainRef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(chain_ref.table_id(), "wizard-mishap");
        assert_eq!(chain_ref.reroll_values(), &[1]);
    }

    #[test]
    fn deserialize_chain_ref_modified_no_reroll() {
        let yaml = r#"
table: some-table
"#;
        let chain_ref: ChainRef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(chain_ref.table_id(), "some-table");
        assert!(chain_ref.reroll_values().is_empty());
    }

    #[test]
    fn deserialize_mixed_chain_list() {
        let yaml = r#"
- animal-type
- table: wizard-mishap
  reroll: [1]
"#;
        let chains: Vec<ChainRef> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[0].table_id(), "animal-type");
        assert!(chains[0].reroll_values().is_empty());
        assert_eq!(chains[1].table_id(), "wizard-mishap");
        assert_eq!(chains[1].reroll_values(), &[1]);
    }

    #[test]
    fn deserialize_simple_table_with_reroll_chain() {
        let yaml = r#"
id: mishap
name: Wizard Mishap
type: simple
tags: []
roll: 1d12
results:
  - min: 1
    max: 1
    text: "Roll twice and combine"
    chain:
      - table: mishap
        reroll: [1]
      - table: mishap
        reroll: [1]
  - min: 2
    max: 12
    text: Other effect
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple { results, .. } => {
                let chains = results[0].chain.as_ref().unwrap();
                assert_eq!(chains.len(), 2);
                assert_eq!(chains[0].table_id(), "mishap");
                assert_eq!(chains[0].reroll_values(), &[1]);
            }
            _ => panic!("Expected Simple table"),
        }
    }

    #[test]
    fn deserialize_manifest_with_files() {
        let yaml = r#"
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
files:
  - path: ../shared/npc-occupation.yaml
    namespace: test.npc
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].namespace, "test.npc");
    }

    #[test]
    fn deserialize_manifest_without_files_defaults_empty() {
        let yaml = r#"
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(manifest.files.is_empty());
    }

    #[test]
    fn deserialize_files_only_manifest() {
        let yaml = r#"
name: Files Only
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
files:
  - path: some-table.yaml
    namespace: test.tables
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(manifest.directories.is_empty());
        assert_eq!(manifest.files.len(), 1);
    }

    #[test]
    fn deserialize_simple_table_default_tags() {
        let yaml = r#"
id: minimal
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
