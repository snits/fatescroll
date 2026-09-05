// ABOUTME: Preparation boundary for result bindings and templates.
// ABOUTME: Checks ordered bindings and scans strict markers without randomness.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::expression::{self, Expression, Value, ValueScope, ValueType};
use crate::models::ResultEntry;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResultTextError {
    pub(crate) location: String,
    pub(crate) offset: usize,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CheckedBinding {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) expression: Expression,
    pub(crate) value_type: ValueType,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Segment {
    Literal(String),
    Expression {
        source: String,
        expression: Expression,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PreparedResult {
    pub(crate) bindings: Vec<CheckedBinding>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) has_text: bool,
}

/// Maximum bindings checked for one result entry.
pub(crate) const MAX_BINDINGS_PER_RESULT: usize = 128;
/// Maximum template source bytes for an entry that uses bindings or strict
/// markers. The rendered-output budget is `MAX_RENDERED_BYTES` below.
pub(crate) const MAX_TEMPLATE_BYTES: usize = 65_536;
/// Maximum rendered-output bytes for an entry that uses bindings or strict
/// markers. Entries on the plain ordinary-dice path stay unbounded.
pub(crate) const MAX_RENDERED_BYTES: usize = 65_536;

/// Names that bindings may not declare. `value` is the built-in lookup and
/// the rest would collide with expression keywords or the dice function.
const RESERVED_NAMES: &[&str] = &["value", "roll", "if", "then", "else", "true", "false"];

/// Binding names follow the expression identifier syntax: ASCII letters and
/// underscores to start, then ASCII letters, digits, or underscores.
fn is_binding_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn prepare(entry: &ResultEntry) -> Result<PreparedResult, ResultTextError> {
    if entry.bindings.len() > MAX_BINDINGS_PER_RESULT {
        return Err(ResultTextError {
            location: "let".to_string(),
            offset: entry.bindings.len(),
            reason: format!(
                "too many bindings: {} exceeds limit of {MAX_BINDINGS_PER_RESULT}",
                entry.bindings.len()
            ),
        });
    }
    // The source budget applies only to entries using bindings or strict
    // markers; ordinary dice text keeps its existing unbounded path.
    if let Some(text) = &entry.text
        && text.len() > MAX_TEMPLATE_BYTES
        && (!entry.bindings.is_empty() || text.contains("{="))
    {
        return Err(ResultTextError {
            location: "text".to_string(),
            offset: MAX_TEMPLATE_BYTES,
            reason: format!("template source exceeds size limit of {MAX_TEMPLATE_BYTES} bytes"),
        });
    }
    // Each binding sees `value` plus earlier bindings only: parse and check
    // against the growing scope, adding the name only after it checks.
    // A self-reference or forward reference is therefore an unknown name.
    let mut scope: BTreeMap<String, ValueType> = BTreeMap::new();
    scope.insert("value".to_string(), ValueType::Integer);
    let mut bindings = Vec::with_capacity(entry.bindings.len());
    for (index, binding) in entry.bindings.iter().enumerate() {
        let location = format!("let[{index}].{}", binding.name);
        if !is_binding_name(&binding.name) {
            return Err(ResultTextError {
                location,
                offset: 0,
                reason: format!("invalid binding name `{}`", binding.name),
            });
        }
        if RESERVED_NAMES.contains(&binding.name.as_str()) {
            return Err(ResultTextError {
                location,
                offset: 0,
                reason: format!("binding name `{}` is reserved", binding.name),
            });
        }
        if scope.contains_key(&binding.name) {
            return Err(ResultTextError {
                location,
                offset: 0,
                reason: format!("duplicate binding name `{}`", binding.name),
            });
        }
        let parsed = expression::parse(&binding.value).map_err(|error| ResultTextError {
            location: location.clone(),
            offset: error.offset,
            reason: error.reason,
        })?;
        let value_type =
            expression::check(&parsed, &scope, true).map_err(|error| ResultTextError {
                location: location.clone(),
                offset: error.offset,
                reason: error.reason,
            })?;
        scope.insert(binding.name.clone(), value_type);
        bindings.push(CheckedBinding {
            name: binding.name.clone(),
            source: binding.value.clone(),
            expression: parsed,
            value_type,
        });
    }
    // The template sees every binding. Strict `{= ...}` openings partition
    // the source before ordinary dice matching: anything outside a strict
    // segment stays a literal here, and Task 4 applies the existing
    // tolerant dice matcher to those literals at render time.
    // Template expressions never roll dice, even in unselected branches.
    let mut segments = Vec::new();
    if let Some(text) = &entry.text {
        for raw in scan_template(text)? {
            match raw {
                RawSegment::Literal(content) => {
                    segments.push(Segment::Literal(content));
                }
                RawSegment::Strict(source) => {
                    let parsed = expression::parse(&source).map_err(|error| ResultTextError {
                        location: "text".to_string(),
                        offset: error.offset,
                        reason: error.reason,
                    })?;
                    expression::check(&parsed, &scope, false).map_err(|error| ResultTextError {
                        location: "text".to_string(),
                        offset: error.offset,
                        reason: error.reason,
                    })?;
                    segments.push(Segment::Expression {
                        source,
                        expression: parsed,
                    });
                }
            }
        }
    }
    Ok(PreparedResult {
        bindings,
        segments,
        has_text: entry.text.is_some(),
    })
}

/// Render a prepared result against the selected lookup value.
///
/// NOTE: the task sketch for this boundary wrote `lookup: i32`; this uses
/// `i64` because expression values are signed 64-bit and `value` inserts as
/// an integer.
///
/// Every binding evaluates in list order into a fresh scope seeded with
/// integer `value`, even when the template is absent or empty (dice draws
/// and runtime errors still surface); absent text then renders as `None`
/// while present-but-empty text renders as `Some("")`. Each source segment renders once left-to-right: literal spans
/// go through the ordinary dice matcher, expression values splice verbatim
/// with no re-matching.
pub(crate) fn render(
    prepared: &PreparedResult,
    lookup: i64,
    rng: &mut impl diceman::Rng,
) -> Result<Option<String>, ResultTextError> {
    let mut scope: ValueScope = BTreeMap::new();
    scope.insert("value".to_string(), Value::Integer(lookup));
    for (index, binding) in prepared.bindings.iter().enumerate() {
        let value = expression::evaluate(&binding.expression, &scope, rng).map_err(|error| {
            ResultTextError {
                location: format!("let[{index}].{}", binding.name),
                offset: error.offset,
                reason: error.reason,
            }
        })?;
        scope.insert(binding.name.clone(), value);
    }
    if prepared.segments.is_empty() {
        if prepared.has_text {
            return Ok(Some(String::new()));
        }
        return Ok(None);
    }
    // The output budget binds exactly the entries the source budget binds:
    // those using bindings or strict markers. The plain ordinary-dice path
    // stays unbounded.
    let budgeted = !prepared.bindings.is_empty()
        || prepared
            .segments
            .iter()
            .any(|segment| matches!(segment, Segment::Expression { .. }));
    let mut out = String::new();
    for segment in &prepared.segments {
        match segment {
            Segment::Literal(span) => append_dice_span(&mut out, span, rng, budgeted)?,
            Segment::Expression { expression, .. } => {
                let value = expression::evaluate(expression, &scope, rng).map_err(|error| {
                    ResultTextError {
                        location: "text".to_string(),
                        offset: error.offset,
                        reason: error.reason,
                    }
                })?;
                match value {
                    Value::Integer(number) => {
                        push_budgeted(&mut out, &number.to_string(), budgeted)?
                    }
                    Value::Boolean(flag) => push_budgeted(&mut out, &flag.to_string(), budgeted)?,
                    Value::Text(text) => push_budgeted(&mut out, &text, budgeted)?,
                }
            }
        }
    }
    Ok(Some(out))
}

/// Append one chunk to the rendered output, failing before the write when
/// the chunk would carry a budgeted entry past the output limit. Unbudgeted
/// entries append directly; output is never built-then-truncated.
fn push_budgeted(out: &mut String, chunk: &str, budgeted: bool) -> Result<(), ResultTextError> {
    if budgeted && out.len() + chunk.len() > MAX_RENDERED_BYTES {
        return Err(ResultTextError {
            location: "text".to_string(),
            offset: MAX_RENDERED_BYTES,
            reason: format!("rendered text exceeds size limit of {MAX_RENDERED_BYTES} bytes"),
        });
    }
    out.push_str(chunk);
    Ok(())
}

/// Ordinary `{dice}` matcher for literal template spans, moved from
/// `roller.rs`: same regex and per-match replacement semantics (failed or
/// nonnumeric dice stay literal), restructured from `replace_all` to an
/// incremental walk so the output budget is checked before every append.
/// Applied per literal span only; spliced expression values are never
/// re-matched.
static DICE_INTERPOLATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^}]+)\}").unwrap());

fn append_dice_span(
    out: &mut String,
    span: &str,
    rng: &mut impl diceman::Rng,
    budgeted: bool,
) -> Result<(), ResultTextError> {
    let mut cursor = 0;
    for mat in DICE_INTERPOLATION.find_iter(span) {
        push_budgeted(out, &span[cursor..mat.start()], budgeted)?;
        // The match is `\{` + group + `\}`; both braces are one byte.
        let expr = &span[mat.start() + 1..mat.end() - 1];
        match diceman::roll_with_rng(expr, rng) {
            Ok(result) => match result.outcome.as_numeric() {
                Some(v) => push_budgeted(out, &v.to_string(), budgeted)?,
                None => push_budgeted(out, mat.as_str(), budgeted)?,
            },
            Err(_) => push_budgeted(out, mat.as_str(), budgeted)?,
        }
        cursor = mat.end();
    }
    push_budgeted(out, &span[cursor..], budgeted)
}

/// A template span as found by the scanner, before type checking.
#[derive(Debug, Clone, PartialEq)]
enum RawSegment {
    Literal(String),
    /// Source text between `{=` and its quote-aware closing brace.
    Strict(String),
}

/// Split template source at strict openings. `{{=` emits a literal `{=` and
/// is recognized before `{=`; everything else outside strict segments stays
/// a literal for the ordinary dice matcher. Returns an error for empty or
/// unterminated markers.
fn scan_template(source: &str) -> Result<Vec<RawSegment>, ResultTextError> {
    let fail = |offset: usize, reason: &str| ResultTextError {
        location: "text".to_string(),
        offset,
        reason: reason.to_string(),
    };
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut index = 0;
    while index < source.len() {
        let rest = &source[index..];
        if rest.starts_with("{{=") {
            // An escaped opening's emitted text is never matched again.
            literal.push_str("{=");
            index += 3;
        } else if rest.starts_with("{=") {
            let end = find_strict_end(source, index + 2)
                .ok_or_else(|| fail(index, "unterminated `{=` marker"))?;
            let inner = source[index + 2..end].to_string();
            if inner.trim().is_empty() {
                return Err(fail(index, "empty `{=` expression"));
            }
            if !literal.is_empty() {
                segments.push(RawSegment::Literal(std::mem::take(&mut literal)));
            }
            segments.push(RawSegment::Strict(inner));
            index = end + 1;
        } else {
            let next = rest.chars().next().unwrap();
            literal.push(next);
            index += next.len_utf8();
        }
    }
    if !literal.is_empty() {
        segments.push(RawSegment::Literal(literal));
    }
    Ok(segments)
}

/// Find the closing brace of a strict marker starting the scan after `{=`.
/// Braces inside double-quoted expression strings are ordinary characters;
/// a backslash inside a string skips the next character. Returns the byte
/// offset of the closing brace, or `None` when unterminated.
fn find_strict_end(source: &str, from: usize) -> Option<usize> {
    let mut index = from;
    let mut in_string = false;
    while index < source.len() {
        let rest = &source[index..];
        let next = rest.chars().next().unwrap();
        if in_string {
            if next == '\\' {
                index += next.len_utf8();
                if let Some(escaped) = source[index..].chars().next() {
                    index += escaped.len_utf8();
                }
                continue;
            }
            if next == '"' {
                in_string = false;
            }
        } else if next == '"' {
            in_string = true;
        } else if next == '}' {
            return Some(index);
        }
        index += next.len_utf8();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ResultBinding;

    #[test]
    fn result_text_rejects_forward_references() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: price
    value: 'count * 25'
  - name: count
    value: 'roll("1d4")'
text: '{= price}'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("count"));
        assert!(error.location.contains("price"));
    }

    #[test]
    fn result_text_accepts_ordered_dependencies() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d4")'
  - name: price
    value: 'count * 25'
text: '{= price}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.bindings.len(), 2);
        assert_eq!(prepared.bindings[0].name, "count");
        assert_eq!(prepared.bindings[0].source, "roll(\"1d4\")");
        assert_eq!(prepared.bindings[1].name, "price");
        assert_eq!(prepared.bindings[1].source, "count * 25");
    }

    #[test]
    fn result_text_rejects_unknown_names() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: price
    value: 'total * 25'
text: '{= price}'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("total"));
        assert!(error.location.contains("price"));
    }

    #[test]
    fn result_text_rejects_duplicate_binding_names() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d4")'
  - name: count
    value: '2'
text: '{= count}'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("count"), "got: {}", error.reason);
        assert!(error.location.contains("let[1]"), "got: {}", error.location);
    }

    #[test]
    fn result_text_rejects_self_references() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: bonus
    value: 'bonus + 1'
text: '{= bonus}'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("bonus"), "got: {}", error.reason);
        assert!(error.location.contains("bonus"), "got: {}", error.location);
    }

    #[test]
    fn result_text_rejects_reserved_binding_names() {
        for name in ["value", "roll", "if", "then", "else", "true", "false"] {
            let yaml =
                format!("min: 1\nmax: 6\nlet:\n  - name: {name}\n    value: '1'\ntext: 'x'\n");
            let entry: ResultEntry = serde_yaml::from_str(&yaml).unwrap();
            let error = prepare(&entry).unwrap_err();
            assert!(
                error.reason.contains(name),
                "for `{name}`, got: {}",
                error.reason
            );
        }
    }

    #[test]
    fn result_text_rejects_invalid_binding_names() {
        for name in ["", "9lives", "has space", "with-dash"] {
            let yaml =
                format!("min: 1\nmax: 6\nlet:\n  - name: '{name}'\n    value: '1'\ntext: 'x'\n");
            let entry: ResultEntry = serde_yaml::from_str(&yaml).unwrap();
            let error = prepare(&entry).unwrap_err();
            assert!(
                error.reason.contains("name"),
                "for `{name}`, got: {}",
                error.reason
            );
        }
    }

    #[test]
    fn result_text_accepts_identifier_binding_names() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: _hidden
    value: '1'
  - name: Count2
    value: '_hidden + 1'
text: 'x'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.bindings.len(), 2);
    }

    #[test]
    fn result_text_scans_pure_marker_without_bindings() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{= value}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert!(prepared.bindings.is_empty());
        assert_eq!(prepared.segments.len(), 1);
        match &prepared.segments[0] {
            Segment::Expression { source, .. } => assert_eq!(source, " value"),
            other => panic!("expected expression segment, got: {other:?}"),
        }
    }

    #[test]
    fn result_text_rejects_dice_in_template() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d4")'
text: '{= roll("1d6") + count}'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("dice"), "got: {}", error.reason);
        assert_eq!(error.location, "text");
    }

    #[test]
    fn result_text_rejects_dice_in_dead_template_branches() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{= if false then roll("1d6") else 1}'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("dice"), "got: {}", error.reason);
        assert_eq!(error.location, "text");
    }

    #[test]
    fn result_text_template_sees_all_bindings() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: first
    value: '1'
  - name: second
    value: 'first + 1'
text: '{= second}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.segments.len(), 1);
        match &prepared.segments[0] {
            Segment::Expression { source, .. } => assert_eq!(source, " second"),
            other => panic!("expected expression segment, got: {other:?}"),
        }
    }

    #[test]
    fn result_text_rejects_unknown_names_in_template() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: 'Found {= total} gold.'
"#,
        )
        .unwrap();
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("total"), "got: {}", error.reason);
        assert_eq!(error.location, "text");
    }

    #[test]
    fn result_text_keeps_literal_braces() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: 'a { b } c {2d6}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(
            prepared.segments,
            vec![Segment::Literal("a { b } c {2d6}".to_string())]
        );
    }

    #[test]
    fn result_text_scans_quoted_braces() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{= if value == 1 then "}" else "{"}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.segments.len(), 1);
        match &prepared.segments[0] {
            Segment::Expression { source, .. } => {
                assert_eq!(source, " if value == 1 then \"}\" else \"{\"")
            }
            other => panic!("expected expression segment, got: {other:?}"),
        }
    }

    #[test]
    fn result_text_marker_takes_precedence_in_braces() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{note {= value}}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.segments.len(), 3);
        assert_eq!(prepared.segments[0], Segment::Literal("{note ".to_string()));
        match &prepared.segments[1] {
            Segment::Expression { source, .. } => assert_eq!(source, " value"),
            other => panic!("expected expression segment, got: {other:?}"),
        }
        assert_eq!(prepared.segments[2], Segment::Literal("}".to_string()));
    }

    #[test]
    fn result_text_keeps_double_brace_dice_literal() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{{1d6}}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(
            prepared.segments,
            vec![Segment::Literal("{{1d6}}".to_string())]
        );
    }

    #[test]
    fn result_text_escapes_double_equals() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{{= value}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(
            prepared.segments,
            vec![Segment::Literal("{= value}".to_string())]
        );
    }

    #[test]
    fn result_text_scans_escaped_then_real_marker() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{{= x} and {= value}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.segments.len(), 2);
        assert_eq!(
            prepared.segments[0],
            Segment::Literal("{= x} and ".to_string())
        );
        match &prepared.segments[1] {
            Segment::Expression { source, .. } => assert_eq!(source, " value"),
            other => panic!("expected expression segment, got: {other:?}"),
        }
    }

    #[test]
    fn result_text_rejects_empty_markers() {
        for text in ["{=}", "{=   }"] {
            let yaml = format!("min: 1\nmax: 6\ntext: '{text}'\n");
            let entry: ResultEntry = serde_yaml::from_str(&yaml).unwrap();
            let error = prepare(&entry).unwrap_err();
            assert!(
                error.reason.contains("empty"),
                "for `{text}`, got: {}",
                error.reason
            );
        }
    }

    #[test]
    fn result_text_rejects_unterminated_markers() {
        for text in ["{= value", "{= \"abc"] {
            let yaml = format!("min: 1\nmax: 6\ntext: '{text}'\n");
            let entry: ResultEntry = serde_yaml::from_str(&yaml).unwrap();
            let error = prepare(&entry).unwrap_err();
            assert!(
                error.reason.contains("unterminated"),
                "for `{text}`, got: {}",
                error.reason
            );
        }
    }

    #[test]
    fn result_text_accepts_all_binding_types() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d4")'
  - name: label
    value: '"gem"'
  - name: plenty
    value: 'count > 3'
text: '{= label}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let types: Vec<ValueType> = prepared
            .bindings
            .iter()
            .map(|binding| binding.value_type)
            .collect();
        assert_eq!(
            types,
            vec![ValueType::Integer, ValueType::Text, ValueType::Boolean]
        );
    }

    #[test]
    fn result_text_handles_absent_and_empty_text() {
        let absent = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![],
            text: None,
            chain: None,
        };
        let prepared = prepare(&absent).unwrap();
        assert!(prepared.segments.is_empty());
        let empty = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![],
            text: Some(String::new()),
            chain: None,
        };
        let prepared = prepare(&empty).unwrap();
        assert!(prepared.segments.is_empty());
    }

    #[test]
    fn result_text_rejects_too_many_bindings() {
        let bindings: Vec<ResultBinding> = (0..129)
            .map(|index| ResultBinding {
                name: format!("b{index}"),
                value: "1".to_string(),
            })
            .collect();
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings,
            text: None,
            chain: None,
        };
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("128"), "got: {}", error.reason);
        let bindings: Vec<ResultBinding> = (0..128)
            .map(|index| ResultBinding {
                name: format!("b{index}"),
                value: "1".to_string(),
            })
            .collect();
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings,
            text: None,
            chain: None,
        };
        assert!(prepare(&entry).is_ok());
    }

    #[test]
    fn result_text_rejects_oversized_binding_expression() {
        let big = format!("1{}", " + 1".repeat(2000));
        assert!(big.len() > 4096);
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![ResultBinding {
                name: "big".to_string(),
                value: big,
            }],
            text: None,
            chain: None,
        };
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("size limit"), "got: {}", error.reason);
    }

    #[test]
    fn result_text_rejects_oversized_template_expression() {
        let big = format!("1{}", " + 1".repeat(2000));
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![],
            text: Some(format!("{{= {big}}}")),
            chain: None,
        };
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("size limit"), "got: {}", error.reason);
    }

    #[test]
    fn result_text_rejects_deep_binding_expression() {
        let deep = format!("{}1", "-".repeat(64));
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![ResultBinding {
                name: "deep".to_string(),
                value: deep,
            }],
            text: None,
            chain: None,
        };
        let error = prepare(&entry).unwrap_err();
        assert!(
            error.reason.contains("depth limit"),
            "got: {}",
            error.reason
        );
    }

    #[test]
    fn result_text_rejects_oversized_template_source() {
        let padding = "x".repeat(70_000);
        // A strict marker opts the entry into the source budget.
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![],
            text: Some(format!("{{= value}}{padding}")),
            chain: None,
        };
        let error = prepare(&entry).unwrap_err();
        assert!(error.reason.contains("exceeds"), "got: {}", error.reason);
        // Bindings alone also opt in, even without a marker.
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![ResultBinding {
                name: "count".to_string(),
                value: "1".to_string(),
            }],
            text: Some(padding.clone()),
            chain: None,
        };
        assert!(prepare(&entry).is_err());
    }

    #[test]
    fn result_text_ignores_template_limit_without_markers_or_bindings() {
        let padding = "x".repeat(70_000);
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![],
            text: Some(format!("plain {padding} {{2d6}}")),
            chain: None,
        };
        let prepared = prepare(&entry).unwrap();
        assert_eq!(prepared.segments.len(), 1);
    }

    #[test]
    fn render_prepared_gem_template() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d1")'
  - name: price
    value: 'count * 25'
text: 'Found {= count} {= if count == 1 then "gem" else "gems"} worth {= price} gold.'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some("Found 1 gem worth 25 gold.".to_string())
        );
    }

    #[test]
    fn render_absent_text_evaluates_bindings_then_returns_none() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d1")'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let before = rng.checkpoint();
        assert_eq!(render(&prepared, 6, &mut rng).unwrap(), None);
        // The unused binding still drew from the caller's RNG.
        assert_ne!(rng.checkpoint(), before);
    }

    #[test]
    fn render_empty_text_evaluates_bindings_then_returns_some_empty() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d1")'
text: ''
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let before = rng.checkpoint();
        assert_eq!(render(&prepared, 6, &mut rng).unwrap(), Some(String::new()));
        assert_ne!(rng.checkpoint(), before);
    }

    #[test]
    fn render_surfaces_binding_errors_without_text() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: '1'
  - name: price
    value: 'count / 0'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let error = render(&prepared, 6, &mut rng).unwrap_err();
        assert_eq!(error.location, "let[1].price");
    }

    #[test]
    fn render_maps_template_runtime_errors_to_text() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: 'Found {= 1 / 0} gold.'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let error = render(&prepared, 6, &mut rng).unwrap_err();
        assert_eq!(error.location, "text");
    }

    #[test]
    fn render_seeds_value_with_lookup() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: doubled
    value: 'value * 2'
text: '{= doubled}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some("12".to_string())
        );
    }

    #[test]
    fn render_stringifies_booleans_and_integers() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: '{= value > 3} {= value}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some("true 6".to_string())
        );
    }

    #[test]
    fn render_splices_string_values_without_rescanning() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: label
    value: '"{1d6}"'
text: 'Found {= label} gold.'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let before = rng.checkpoint();
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some("Found {1d6} gold.".to_string())
        );
        // No dice were rolled for the spliced string.
        assert_eq!(rng.checkpoint(), before);
    }

    #[test]
    fn render_applies_ordinary_dice_per_literal_span() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d1")'
text: 'Found {2d6} gold and {= count} gem.'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(42);
        let mut manual = diceman::FastRng::with_seed(42);
        // Bindings evaluate first: `roll("1d1")` draws once from the caller RNG.
        let _ = diceman::roller::evaluate_with_rng(&diceman::parse("1d1").unwrap(), &mut manual)
            .unwrap();
        let dice = diceman::roll_with_rng("2d6", &mut manual).unwrap();
        let expected = format!(
            "Found {} gold and 1 gem.",
            dice.outcome.as_numeric().unwrap()
        );
        assert_eq!(render(&prepared, 6, &mut rng).unwrap(), Some(expected));
        assert_eq!(rng.checkpoint(), manual.checkpoint());
    }

    #[test]
    fn render_preserves_non_dice_braces_and_double_braces() {
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
text: 'Hello {world} and {{1d6}} end'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(42);
        let rendered = render(&prepared, 6, &mut rng).unwrap().unwrap();
        assert!(rendered.contains("{world}"), "got: {rendered}");
        assert!(rendered.contains("{{1d6}}"), "got: {rendered}");
    }

    #[test]
    fn render_budgets_repeated_values_near_boundary() {
        let pad = "x".repeat(4000);
        let entry = |repeats: usize| ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![ResultBinding {
                name: "pad".to_string(),
                value: format!("\"{pad}\""),
            }],
            text: Some("{= pad} ".repeat(repeats)),
            chain: None,
        };
        // 16 repeats render 64,016 bytes and fit the 65,536 budget.
        let prepared = prepare(&entry(16)).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let rendered = render(&prepared, 6, &mut rng).unwrap().unwrap();
        assert_eq!(rendered.len(), 16 * 4001);
        // 17 repeats render 68,017 bytes and must fail before appending past
        // the budget, never build-then-truncate.
        let prepared = prepare(&entry(17)).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let error = render(&prepared, 6, &mut rng).unwrap_err();
        assert_eq!(error.location, "text");
    }

    #[test]
    fn render_budgets_dice_expansion_past_boundary() {
        // Near-full output (65,500 bytes) from a literal plus one value,
        // then deterministic `{1d1}` expansions carry it past the budget.
        let pad = "y".repeat(472);
        let entry = |dice: usize| ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![ResultBinding {
                name: "pad".to_string(),
                value: format!("\"{pad}\""),
            }],
            text: Some(format!(
                "{}{}{}",
                "x".repeat(65_028),
                "{= pad}",
                "{1d1}".repeat(dice)
            )),
            chain: None,
        };
        // 65,028 + 472 renders 65,500 bytes and fits.
        let prepared = prepare(&entry(0)).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let rendered = render(&prepared, 6, &mut rng).unwrap().unwrap();
        assert_eq!(rendered.len(), 65_500);
        // Sixty one-byte expansions would render 65,560 bytes and must fail
        // at a dice append, never build-then-truncate.
        let prepared = prepare(&entry(60)).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let error = render(&prepared, 6, &mut rng).unwrap_err();
        assert_eq!(error.location, "text");
    }

    #[test]
    fn render_leaves_plain_text_unbudgeted() {
        // Entries without bindings or strict markers keep the existing
        // unbounded ordinary-dice path.
        let entry = ResultEntry {
            min: 1,
            max: 6,
            bindings: vec![],
            text: Some("x".repeat(70_000)),
            chain: None,
        };
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(11);
        let rendered = render(&prepared, 6, &mut rng).unwrap().unwrap();
        assert_eq!(rendered.len(), 70_000);
    }

    #[test]
    fn render_binding_used_twice_draws_once() {
        // `count` evaluates once into the scope; two template uses must not
        // draw again. One explicit diceman 1d1 draw is the oracle for both
        // the value and the consumed RNG state.
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: count
    value: 'roll("1d1")'
text: '{= count} and {= count}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(7);
        let mut manual = diceman::FastRng::with_seed(7);
        let drawn =
            diceman::roller::evaluate_with_rng(&diceman::parse("1d1").unwrap(), &mut manual)
                .unwrap()
                .outcome
                .as_numeric()
                .unwrap();
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some(format!("{drawn} and {drawn}"))
        );
        assert_eq!(rng.checkpoint(), manual.checkpoint());
    }

    #[test]
    fn render_unused_binding_draws_exactly_once() {
        // `stashed` never appears in the template but still evaluates and
        // draws exactly once from the caller's RNG; rendering still succeeds.
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: stashed
    value: 'roll("1d1")'
text: 'plain'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(7);
        let mut manual = diceman::FastRng::with_seed(7);
        let _ = diceman::roller::evaluate_with_rng(&diceman::parse("1d1").unwrap(), &mut manual)
            .unwrap();
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some("plain".to_string())
        );
        assert_eq!(rng.checkpoint(), manual.checkpoint());
    }

    #[test]
    fn render_lazy_binding_branch_draws_nothing() {
        // Binding-level laziness: the unselected `roll("1d1")` branch never
        // draws, so the whole render leaves the RNG untouched.
        let entry: ResultEntry = serde_yaml::from_str(
            r#"
min: 1
max: 6
let:
  - name: x
    value: 'if false then roll("1d1") else 42'
text: '{= x}'
"#,
        )
        .unwrap();
        let prepared = prepare(&entry).unwrap();
        let mut rng = diceman::FastRng::with_seed(7);
        let before = rng.checkpoint();
        assert_eq!(
            render(&prepared, 6, &mut rng).unwrap(),
            Some("42".to_string())
        );
        assert_eq!(rng.checkpoint(), before);
    }
}
