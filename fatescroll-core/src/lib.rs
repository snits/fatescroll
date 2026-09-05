// ABOUTME: Public API for the fatescroll random table library.
// ABOUTME: Re-exports core types and provides top-level convenience functions.

pub mod collection;
pub mod dice;
pub mod display;
pub mod error;
pub mod fixer;
pub mod init;
pub mod loader;
pub mod models;
pub mod registry;
pub mod roller;
pub mod search;
pub mod validator;

#[cfg(test)]
pub(crate) mod test_utils;

#[cfg(test)]
mod expression;

use std::path::Path;

pub use error::Error;
pub use loader::{build_registry, load_table, load_table_str};
pub use models::{RollResult, Table};
pub use registry::Registry;

/// Load and validate a collection from a manifest file path.
pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    let registry = loader::load_collection(manifest_path)?;

    if let Err(errors) = validator::validate_references(&registry) {
        return Err(error::LoadError::Multiple {
            errors: errors.into_iter().map(Error::from).collect(),
        }
        .into());
    }

    Ok(registry)
}
