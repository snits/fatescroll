// ABOUTME: WASM bindings exposing fatescroll-core validation, dice info, and rolling
// ABOUTME: to the Table Forge webui. All I/O is JSON strings; RNG seeds come from JS.

use fatescroll_core::collection::CollectionFile;
use fatescroll_core::models::{Manifest, ModifierRange, Table};
use fatescroll_core::validator::{EnvelopeError, outcome_envelope, validate_references};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
struct FileInput {
    path: String,
    namespace: String,
    stem: String,
    contents: String,
}

fn parse_inputs(
    manifest_yaml: &str,
    files_json: &str,
) -> Result<(Manifest, Vec<CollectionFile>), String> {
    let manifest: Manifest =
        serde_yaml::from_str(manifest_yaml).map_err(|e| format!("manifest: {e}"))?;
    let inputs: Vec<FileInput> =
        serde_json::from_str(files_json).map_err(|e| format!("files: {e}"))?;
    let files = inputs
        .into_iter()
        .map(|f| CollectionFile {
            path: PathBuf::from(f.path),
            namespace: f.namespace,
            stem: f.stem,
            contents: f.contents,
        })
        .collect();
    Ok((manifest, files))
}

/// Validate a whole collection held in memory. Returns {"errors": [String]}.
/// Runs the same per-file checks as the CLI loader, then cross-reference
/// checks — always both, so multi-error collections report a superset of what
/// the CLI (which stops before cross-reference checks on load errors) shows.
#[wasm_bindgen]
pub fn validate_collection(manifest_yaml: &str, files_json: &str) -> String {
    let (manifest, files) = match parse_inputs(manifest_yaml, files_json) {
        Ok(v) => v,
        Err(e) => return json!({ "errors": [e] }).to_string(),
    };
    let (registry, errors) = fatescroll_core::build_registry(&manifest, &files);
    let mut messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
    if let Err(ref_errors) = validate_references(&registry) {
        messages.extend(ref_errors.iter().map(|e| e.to_string()));
    }
    json!({ "errors": messages }).to_string()
}

/// Dice expression info for the editor's roll-input hint.
/// {"ok":true,"kind":"digit"|"range"|"simulated","min":i64,"max":i64,"outcomes":usize}
/// or {"ok":false,"reason":String}. For "range"/"simulated", `outcomes` is the
/// envelope width (max-min+1), not true cardinality for gappy distributions.
#[wasm_bindgen]
pub fn dice_info(expr: &str) -> String {
    let parsed = match diceman::parse(expr) {
        Ok(p) => p,
        Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
    };
    if let Some((sides, count)) = fatescroll_core::dice::digit_dice_params(&parsed) {
        let values = fatescroll_core::dice::digit_dice_values(sides, count);
        return json!({
            "ok": true, "kind": "digit",
            "min": values.first().copied(), "max": values.last().copied(),
            "outcomes": values.len(),
        })
        .to_string();
    }
    match fatescroll_core::dice::dice_range(expr) {
        Ok((min, max)) => json!({
            "ok": true, "kind": "range", "min": min, "max": max,
            "outcomes": (max - min + 1),
        })
        .to_string(),
        // Analytically unsupported (keep/drop, exploding, ...): fall back to
        // the same seeded simulation the validator uses for envelopes.
        Err(_) => match diceman::simulate_seeded(expr, 100_000, 42) {
            Ok(sim) => json!({
                "ok": true, "kind": "simulated", "min": sim.min, "max": sim.max,
                "outcomes": (sim.max - sim.min + 1),
            })
            .to_string(),
            Err(e) => json!({ "ok": false, "reason": e.to_string() }).to_string(),
        },
    }
}

/// Expected coverage values for a table's results, so autofill can never
/// produce ranges the validator rejects. Digit dice (D66, ...) yield their
/// exact non-contiguous values (a modifier on them is an error); every other
/// expression's envelope comes from the validator's own
/// `fatescroll_core::validator::outcome_envelope` — analytic when a modifier
/// is present, seeded simulation otherwise, rejecting reversed modifier
/// ranges, negative values, and over-wide envelopes exactly as validate_table
/// does. {"ok":true,"values":[i64]} or {"ok":false,"reason":String}.
#[wasm_bindgen]
pub fn expected_values(expr: &str, mod_on: bool, mod_min: i32, mod_max: i32) -> String {
    let parsed = match diceman::parse(expr) {
        Ok(p) => p,
        Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
    };
    if let Some((sides, count)) = fatescroll_core::dice::digit_dice_params(&parsed) {
        if mod_on {
            return json!({ "ok": false, "reason": "modifier_range unsupported for digit dice" })
                .to_string();
        }
        return json!({ "ok": true, "values": fatescroll_core::dice::digit_dice_values(sides, count) })
            .to_string();
    }
    let modifier = mod_on.then_some(ModifierRange {
        min: mod_min,
        max: mod_max,
    });
    match outcome_envelope(expr, modifier) {
        Ok((lo, hi)) => {
            json!({ "ok": true, "values": (lo..=hi).collect::<Vec<i32>>() }).to_string()
        }
        Err(e) => json!({ "ok": false, "reason": envelope_reason(e) }).to_string(),
    }
}

/// Human-readable reason for an envelope failure, carrying the numbers the
/// editor needs to explain the rejection.
fn envelope_reason(err: EnvelopeError) -> String {
    match err {
        EnvelopeError::ModifierReversed { min, max } => {
            format!("modifier_range reversed: [{min}, {max}]")
        }
        EnvelopeError::Dice(e) => e.to_string(),
        EnvelopeError::NegativeValues { min, max } => {
            format!("dice range [{min}, {max}] includes negative values")
        }
        EnvelopeError::TooWide { width, max } => {
            format!("envelope too wide: {width} > {max}")
        }
    }
}

/// Probability distribution for the editor's probability pills, from the same
/// seeded simulation the validator uses. Returns outcomes sorted by value:
/// {"ok":true,"outcomes":[[value, probability], ...]} or {"ok":false,"reason":...}.
#[wasm_bindgen]
pub fn histogram(expr: &str) -> String {
    match diceman::simulate_seeded(expr, 100_000, 42) {
        Ok(sim) => {
            let n = sim.n as f64;
            let outcomes: Vec<(i64, f64)> = sim
                .sorted_outcomes()
                .into_iter()
                .map(|(v, count)| (v, count as f64 / n))
                .collect();
            json!({ "ok": true, "outcomes": outcomes }).to_string()
        }
        Err(e) => json!({ "ok": false, "reason": e.to_string() }).to_string(),
    }
}

/// Roll a table (by FQID) against the in-memory collection. Returns the
/// serialized RollResult tree, or {"error": String}. Tables dropped for
/// validation errors roll as not-found, so callers should surface
/// validate_collection results before rolling.
#[wasm_bindgen]
pub fn roll_collection(manifest_yaml: &str, files_json: &str, fqid: &str, seed: u64) -> String {
    let (manifest, files) = match parse_inputs(manifest_yaml, files_json) {
        Ok(v) => v,
        Err(e) => return json!({ "error": e }).to_string(),
    };
    // Roll best-effort: broken tables were dropped by build_registry; the
    // roller reports unresolved references itself.
    let (registry, _errors) = fatescroll_core::build_registry(&manifest, &files);
    let mut rng = diceman::FastRng::with_seed(seed);
    match fatescroll_core::roller::roll_with_rng(&registry, fqid, &mut rng) {
        Ok(result) => serde_json::to_string(&result)
            .unwrap_or_else(|e| json!({ "error": e.to_string() }).to_string()),
        Err(e) => json!({ "error": e.to_string() }).to_string(),
    }
}

#[derive(Deserialize)]
struct RawFile {
    path: String,
    contents: String,
}

/// `"core/"` and `"."` normalize to `"core"` / `""` so zip paths compare
/// against manifest directory entries verbatim.
fn normalize_dir(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed == "." { "" } else { trimmed }
}

/// `"core/oracle.yaml"` -> `("core", "oracle.yaml")`; `"oracle.yaml"` -> `("", "oracle.yaml")`.
fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn is_yaml_name(name: &str) -> bool {
    name.ends_with(".yaml") || name.ends_with(".yml")
}

fn is_manifest_name(name: &str) -> bool {
    name == "manifest.yaml" || name.ends_with(".manifest.yaml")
}

/// Parse a whole collection held in memory for import into the editor.
/// Takes ALL ingested files; discovery mirrors the on-disk loader
/// (collection.rs): non-recursive per `directories:` entry, `.yaml`/`.yml`
/// only, manifests skipped. Tables must carry an `id` matching the filename
/// stem, exactly as `build_registry` enforces. Does NOT run validate_table —
/// semantically invalid tables load fine and get fixed in the editor.
/// Duplicate-FQID detection is not mirrored either (the editor's live
/// validate_collection catches collisions after load). Directory paths must
/// be plain relative segments ("core/deep"); "./core" forms won't match.
/// Returns {"manifest": .., "tables": [{path, namespace, stem, table}],
/// "ignored_yaml": [..]} or {"errors": [String]} (all-or-nothing).
#[wasm_bindgen]
pub fn parse_collection(manifest_yaml: &str, files_json: &str) -> String {
    let manifest: Manifest = match serde_yaml::from_str(manifest_yaml) {
        Ok(m) => m,
        Err(e) => return json!({ "errors": [format!("manifest: {e}")] }).to_string(),
    };
    if !manifest.files.is_empty() {
        return json!({ "errors": ["manifest: `files:` entries are not supported by Table Forge"] })
            .to_string();
    }
    let raw_files: Vec<RawFile> = match serde_json::from_str(files_json) {
        Ok(v) => v,
        Err(e) => return json!({ "errors": [format!("files: {e}")] }).to_string(),
    };

    let mut errors: Vec<String> = Vec::new();
    let mut tables: Vec<serde_json::Value> = Vec::new();
    let mut matched = vec![false; raw_files.len()];

    for dir in &manifest.directories {
        let dir_path = dir.path.to_string_lossy();
        let dir_norm = normalize_dir(&dir_path);
        for (i, f) in raw_files.iter().enumerate() {
            let (parent, name) = split_parent(&f.path);
            if parent != dir_norm || !is_yaml_name(name) {
                continue;
            }
            matched[i] = true;
            if is_manifest_name(name) {
                continue;
            }
            let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
            match serde_yaml::from_str::<Table>(&f.contents) {
                Ok(table) => {
                    if table.id() != stem {
                        errors.push(format!(
                            "{}: table id '{}' does not match filename '{}'",
                            f.path,
                            table.id(),
                            stem
                        ));
                    } else {
                        tables.push(json!({
                            "path": f.path,
                            "namespace": dir.namespace,
                            "stem": stem,
                            "table": table,
                        }));
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", f.path)),
            }
        }
    }

    if !errors.is_empty() {
        return json!({ "errors": errors }).to_string();
    }

    let ignored_yaml: Vec<&str> = raw_files
        .iter()
        .enumerate()
        .filter(|(i, f)| !matched[*i] && is_yaml_name(&f.path))
        .map(|(_, f)| f.path.as_str())
        .collect();

    let directories: Vec<serde_json::Value> = manifest
        .directories
        .iter()
        .map(|d| json!({ "path": d.path.to_string_lossy(), "namespace": d.namespace }))
        .collect();

    json!({
        "manifest": {
            "name": manifest.name,
            "version": manifest.version,
            "namespace": manifest.namespace,
            "author": manifest.author,
            "min_tool_version": manifest.min_tool_version,
            "directories": directories,
        },
        "tables": tables,
        "ignored_yaml": ignored_yaml,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "name: T\nversion: \"1.0\"\nnamespace: t\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: core\n    namespace: t.core\n";

    fn files_json() -> String {
        serde_json::json!([{
            "path": "core/oracle.yaml", "namespace": "t.core", "stem": "oracle",
            "contents": "id: oracle\nname: Oracle\ntype: simple\nroll: 1d6\nresults:\n  - min: 1\n    max: 6\n    text: \"Yes\"\n"
        }]).to_string()
    }

    #[test]
    fn validate_collection_valid() {
        let out: serde_json::Value =
            serde_json::from_str(&validate_collection(MANIFEST, &files_json())).unwrap();
        assert_eq!(out["errors"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn validate_collection_reports_unresolved_chain() {
        let files = serde_json::json!([{
            "path": "core/a.yaml", "namespace": "t.core", "stem": "a",
            "contents": "id: a\nname: A\ntype: simple\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: X\n    chain:\n      - missing-table\n"
        }]).to_string();
        let out: serde_json::Value =
            serde_json::from_str(&validate_collection(MANIFEST, &files)).unwrap();
        let errs = out["errors"].as_array().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].as_str().unwrap().contains("missing-table"));
    }

    #[test]
    fn dice_info_standard_and_digit_and_bad() {
        let d6: serde_json::Value = serde_json::from_str(&dice_info("2d6")).unwrap();
        assert_eq!(
            (
                d6["ok"].as_bool(),
                d6["min"].as_i64(),
                d6["max"].as_i64(),
                d6["kind"].as_str()
            ),
            (Some(true), Some(2), Some(12), Some("range"))
        );
        let d66: serde_json::Value = serde_json::from_str(&dice_info("D66")).unwrap();
        assert_eq!(
            (
                d66["kind"].as_str(),
                d66["min"].as_i64(),
                d66["max"].as_i64(),
                d66["outcomes"].as_i64()
            ),
            (Some("digit"), Some(11), Some(66), Some(36))
        );
        let bad: serde_json::Value = serde_json::from_str(&dice_info("not dice")).unwrap();
        assert_eq!(bad["ok"].as_bool(), Some(false));
        assert!(bad["reason"].is_string());
    }

    #[test]
    fn expected_values_modifier_and_digit() {
        let v: serde_json::Value =
            serde_json::from_str(&expected_values("1d8", true, 0, 6)).unwrap();
        let vals: Vec<i64> = v["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap())
            .collect();
        assert_eq!(vals, (1..=14).collect::<Vec<i64>>());
        let d66: serde_json::Value =
            serde_json::from_str(&expected_values("D66", false, 0, 0)).unwrap();
        let dv = d66["values"].as_array().unwrap();
        assert_eq!(dv.len(), 36);
        assert_eq!(dv[6].as_i64(), Some(21)); // 11..16 then 21
        // digit dice + modifier is a core error
        let err: serde_json::Value =
            serde_json::from_str(&expected_values("D66", true, 0, 1)).unwrap();
        assert_eq!(err["ok"].as_bool(), Some(false));
    }

    #[test]
    fn histogram_probabilities_sum_to_one() {
        let h: serde_json::Value = serde_json::from_str(&histogram("2d6")).unwrap();
        let outcomes = h["outcomes"].as_array().unwrap();
        let total: f64 = outcomes.iter().map(|o| o[1].as_f64().unwrap()).sum();
        assert!((total - 1.0).abs() < 1e-9, "probabilities sum to {total}");
        // sorted by value, min 2 max 12
        assert_eq!(outcomes.first().unwrap()[0].as_i64(), Some(2));
        assert_eq!(outcomes.last().unwrap()[0].as_i64(), Some(12));
    }

    #[test]
    fn expected_values_rejects_modifier_on_unsupported_dice() {
        // validate_table's modifier branch is analytic-only (dice_range); keep/drop
        // dice with a modifier are invalid tables, so autofill must refuse too.
        let err: serde_json::Value =
            serde_json::from_str(&expected_values("4d6kh3", true, 0, 1)).unwrap();
        assert_eq!(err["ok"].as_bool(), Some(false));
    }

    #[test]
    fn expected_values_rejects_reversed_modifier_range() {
        // validate_table rejects mod_min > mod_max (ModifierRangeReversed),
        // so autofill must refuse too — even when the reversal is smaller
        // than the dice span.
        let err: serde_json::Value =
            serde_json::from_str(&expected_values("1d8", true, 5, 0)).unwrap();
        assert_eq!(err["ok"].as_bool(), Some(false));
        assert!(err["reason"].as_str().unwrap().contains('5'));
    }

    #[test]
    fn validate_collection_malformed_inputs_report_one_error() {
        // Pins the {"errors":[...]} shape the TS client branches on.
        let bad_manifest: serde_json::Value =
            serde_json::from_str(&validate_collection(": not [ yaml", &files_json())).unwrap();
        let errs = bad_manifest["errors"].as_array().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].as_str().unwrap().starts_with("manifest:"));

        let bad_files: serde_json::Value =
            serde_json::from_str(&validate_collection(MANIFEST, "{ not json")).unwrap();
        let errs = bad_files["errors"].as_array().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].as_str().unwrap().starts_with("files:"));
    }

    #[test]
    fn dice_info_keep_modifier_falls_back_to_simulation() {
        let kh: serde_json::Value = serde_json::from_str(&dice_info("4d6kh3")).unwrap();
        assert_eq!(kh["ok"].as_bool(), Some(true));
        assert_eq!(kh["kind"].as_str(), Some("simulated"));
        assert_eq!(kh["min"].as_i64(), Some(3));
        assert_eq!(kh["max"].as_i64(), Some(18));
    }

    #[test]
    fn expected_values_rejects_negative_envelope() {
        // validate_table rejects dice whose simulated envelope includes negatives.
        let err: serde_json::Value =
            serde_json::from_str(&expected_values("1d6 - 3", false, 0, 0)).unwrap();
        assert_eq!(err["ok"].as_bool(), Some(false));
    }

    #[test]
    fn roll_collection_returns_result_tree() {
        let out: serde_json::Value = serde_json::from_str(&roll_collection(
            MANIFEST,
            &files_json(),
            "t.core.oracle",
            7,
        ))
        .unwrap();
        assert_eq!(out["table_name"].as_str(), Some("Oracle"));
        assert!(out["roll"].as_i64().is_some());
    }

    #[test]
    fn roll_collection_unknown_table_is_error() {
        let out: serde_json::Value =
            serde_json::from_str(&roll_collection(MANIFEST, &files_json(), "t.core.nope", 7))
                .unwrap();
        assert!(out["error"].is_string());
    }

    const OPEN_MANIFEST: &str = "name: T\nversion: \"1.0\"\nnamespace: t\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: core/\n    namespace: t.core\n  - path: core/deep\n    namespace: t.core.deep\n";

    const ORACLE_YAML: &str = "id: oracle\nname: Oracle\ntype: simple\nroll: 1d6\nresults:\n  - min: 1\n    max: 6\n    text: \"Yes\"\n";

    #[test]
    fn parse_collection_discovers_and_parses() {
        let files = serde_json::json!([
            {"path": "core/oracle.yaml", "contents": ORACLE_YAML},
            {"path": "core/deep/combo.yaml", "contents": "id: combo\nname: Combo\ntype: compound\ntables:\n  - oracle\n"},
            {"path": "core/deep/nested/too-deep.yaml", "contents": "id: too-deep\nname: X\ntype: compound\ntables: []\n"},
            {"path": "elsewhere/stray.yaml", "contents": "id: stray\nname: X\ntype: compound\ntables: []\n"},
            {"path": "core/readme.txt", "contents": "not yaml"},
            {"path": "core/other.manifest.yaml", "contents": "name: nested\n"}
        ])
        .to_string();
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, &files)).unwrap();
        assert!(out.get("errors").is_none(), "unexpected errors: {out}");

        assert_eq!(out["manifest"]["name"], "T");
        assert_eq!(out["manifest"]["namespace"], "t");
        assert!(out["manifest"]["author"].is_null());
        let dirs = out["manifest"]["directories"].as_array().unwrap();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0]["path"], "core/");
        assert_eq!(dirs[0]["namespace"], "t.core");

        // Discovery is non-recursive: nested/too-deep.yaml and elsewhere/
        // stray.yaml are not loaded; *.manifest.yaml and non-yaml are skipped.
        let tables = out["tables"].as_array().unwrap();
        assert_eq!(tables.len(), 2);
        let oracle = tables.iter().find(|t| t["stem"] == "oracle").unwrap();
        assert_eq!(oracle["path"], "core/oracle.yaml");
        assert_eq!(oracle["namespace"], "t.core");
        assert_eq!(oracle["table"]["type"], "simple");
        assert_eq!(oracle["table"]["roll"], "1d6");
        assert_eq!(oracle["table"]["results"][0]["text"], "Yes");
        let combo = tables.iter().find(|t| t["stem"] == "combo").unwrap();
        assert_eq!(combo["namespace"], "t.core.deep");
        assert_eq!(combo["table"]["tables"][0], "oracle");

        // Unloaded yaml is warned about; *.manifest.yaml inside a listed dir
        // is silently skipped (mirrors on-disk discovery), non-yaml ignored.
        let ignored: Vec<&str> = out["ignored_yaml"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            ignored,
            vec!["core/deep/nested/too-deep.yaml", "elsewhere/stray.yaml"]
        );
    }

    #[test]
    fn parse_collection_serializes_chain_forms() {
        let files = serde_json::json!([{
            "path": "core/a.yaml",
            "contents": "id: a\nname: A\ntype: simple\nroll: 1d4\nmodifier_range: [0, 2]\nresults:\n  - min: 1\n    max: 6\n    text: X\n    chain:\n      - plain-ref\n      - table: a\n        reroll: [1]\n"
        }])
        .to_string();
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, &files)).unwrap();
        assert!(out.get("errors").is_none(), "unexpected errors: {out}");
        let table = &out["tables"][0]["table"];
        assert_eq!(table["modifier_range"], serde_json::json!([0, 2]));
        let chain = table["results"][0]["chain"].as_array().unwrap();
        assert_eq!(chain[0], "plain-ref");
        assert_eq!(chain[1]["table"], "a");
        assert_eq!(chain[1]["reroll"], serde_json::json!([1]));
    }
}
