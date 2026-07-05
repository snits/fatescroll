// ABOUTME: WASM bindings exposing fatescroll-core validation, dice info, and rolling
// ABOUTME: to the Table Forge webui. All I/O is JSON strings; RNG seeds come from JS.

use fatescroll_core::collection::CollectionFile;
use fatescroll_core::models::Manifest;
use fatescroll_core::validator::validate_references;
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

/// Expected coverage values for a table's results, mirroring validate_table's
/// exact branching (validator.rs:90-159) so autofill can never produce ranges
/// the validator rejects:
/// - digit dice -> exact digit values; modifier -> error
/// - modifier on -> analytic dice_range ONLY (its error propagates; no
///   simulation fallback — validate_table rejects such tables)
/// - no modifier -> simulate_seeded envelope, rejecting negatives like the
///   validator does
/// {"ok":true,"values":[i64]} or {"ok":false,"reason":String}.
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
    let (lo, hi) = if mod_on {
        match fatescroll_core::dice::dice_range(expr) {
            Ok((dmin, dmax)) => (dmin as i64 + mod_min as i64, dmax as i64 + mod_max as i64),
            Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
        }
    } else {
        match diceman::simulate_seeded(expr, 100_000, 42) {
            Ok(sim) if sim.min < 0 || sim.max < 0 => {
                return json!({ "ok": false, "reason": "dice range includes negative values" })
                    .to_string();
            }
            Ok(sim) => (sim.min, sim.max),
            Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
        }
    };
    if lo > hi || hi - lo > 100_000 {
        return json!({ "ok": false, "reason": "envelope reversed or too wide" }).to_string();
    }
    json!({ "ok": true, "values": (lo..=hi).collect::<Vec<i64>>() }).to_string()
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
/// serialized RollResult tree, or {"error": String}.
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
}
