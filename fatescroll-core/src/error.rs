// ABOUTME: Error types for fatescroll operations.
// ABOUTME: Covers validation, loading, rolling, and search errors.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("loading error: {0}")]
    Load(#[from] LoadError),

    #[error("roll error: {0}")]
    Roll(#[from] RollError),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("dice error: {0}")]
    Dice(#[from] diceman::Error),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("range reversed: min {min} > max {max} in table '{table}'")]
    RangeReversed { table: String, min: u32, max: u32 },

    #[error(
        "entry range [{entry_min}..{entry_max}] outside dice range [{dice_min}..{dice_max}] in table '{table}'"
    )]
    EntryOutOfRange {
        table: String,
        entry_min: u32,
        entry_max: u32,
        dice_min: u32,
        dice_max: u32,
    },

    #[error("range gap in table '{table}': missing values {missing:?}")]
    RangeGap { table: String, missing: Vec<u32> },

    #[error("range overlap in table '{table}': values {overlapping:?} covered multiple times")]
    RangeOverlap {
        table: String,
        overlapping: Vec<u32>,
    },

    #[error("invalid dice expression '{expr}' in table '{table}': {reason}")]
    InvalidDiceExpression {
        table: String,
        expr: String,
        reason: String,
    },

    #[error("unsupported dice expression '{expr}': {reason}")]
    UnsupportedDiceExpression { expr: String, reason: String },

    #[error("invalid namespace '{namespace}': {reason}")]
    InvalidNamespace { namespace: String, reason: String },

    #[error("directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("unresolved chain reference '{reference}' in table '{table}'")]
    UnresolvedChain { table: String, reference: String },

    #[error("unresolved compound table reference '{reference}' in table '{table}'")]
    UnresolvedCompoundRef { table: String, reference: String },

    #[error("duplicate table ID '{id}'")]
    DuplicateId { id: String },

    #[error("file entry not found: {path}")]
    FileEntryNotFound { path: PathBuf },

    #[error("file entry is not a file: {path}")]
    FileEntryNotAFile { path: PathBuf },

    #[error("file entry has invalid extension (expected .yaml or .yml): {path}")]
    FileEntryInvalidExtension { path: PathBuf },

    #[error("table id '{id}' does not match filename '{filename}' in {path}")]
    IdFilenameMismatch {
        id: String,
        filename: String,
        path: PathBuf,
    },
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("manifest not found at {path}")]
    ManifestNotFound { path: PathBuf },

    #[error("failed to read file {path}: {reason}")]
    FileRead { path: PathBuf, reason: String },

    #[error("invalid format in {path}: {reason}")]
    InvalidFormat { path: PathBuf, reason: String },

    #[error("multiple errors during load:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    Multiple { errors: Vec<Error> },
}

#[derive(Debug, Error)]
pub enum RollError {
    #[error("table not found: '{id}'")]
    TableNotFound { id: String },

    #[error("roll value {value} out of range for table '{table}'")]
    RollOutOfRange { table: String, value: i64 },

    #[error("chain depth limit ({limit}) exceeded at table '{table}'")]
    ChainDepthExceeded { table: String, limit: usize },

    #[error("negative dice result ({value}) not supported")]
    NegativeRoll { value: i64 },

    #[error("dice evaluation failed for '{expr}' in table '{table}': {reason}")]
    DiceEvaluation {
        table: String,
        expr: String,
        reason: String,
    },

    #[error(
        "reroll attempts exhausted ({attempts}) for table '{table}' with reroll values {reroll_values:?}"
    )]
    RerollExhausted {
        table: String,
        attempts: usize,
        reroll_values: Vec<u32>,
    },
}
