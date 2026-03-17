// ABOUTME: Shared test utilities for fatescroll-core.
// ABOUTME: Provides common helpers like fixture path resolution.

use std::path::PathBuf;

pub fn fixtures_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name)
}
