// ABOUTME: Public API for the fatescroll random table library.
// ABOUTME: Re-exports core types and provides top-level convenience functions.

pub mod collection;
pub mod error;
pub mod fixer;
pub mod loader;
pub mod models;
pub mod registry;
pub mod roller;
pub mod search;
pub mod validator;

use std::path::Path;

pub use error::Error;
pub use models::{RollResult, Table};
pub use registry::Registry;

/// Load and validate a collection from a directory containing manifest.yaml.
pub fn load_collection(collection_dir: &Path) -> Result<Registry, Error> {
    let manifest_path = collection_dir.join("manifest.yaml");
    let registry = loader::load_collection(&manifest_path)?;

    if let Err(errors) = validator::validate_references(&registry) {
        return Err(error::LoadError::Multiple {
            errors: errors.into_iter().map(Error::from).collect(),
        }
        .into());
    }

    Ok(registry)
}
