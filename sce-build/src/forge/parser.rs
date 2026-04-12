// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Forge parser — extracts kind-specific models from Extended SCXML.
//
// Reads `sce:kind` on <scxml> root and dispatches to kind-specific parsing.
// Also handles inline kinds on <data> elements within statechart documents.

use crate::forge::error::{ForgeError, ValidationError, XmlError};
use crate::forge::model::*;

/// Detect the `sce:kind` attribute on the <scxml> root element.
/// Returns `None` if no `sce:kind` is present (defaults to statechart).
pub fn detect_kind(content: &str) -> Result<Option<ForgeKind>, ForgeError> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| XmlError::Parse(e.to_string()))?;
    let root = doc.root_element();
    Ok(detect_kind_from_node(&root)?)
}

/// Single-parse entry point: detect kind and parse forge document in one pass.
/// Returns `None` if the document is a statechart (no `sce:kind` or `sce:kind="statechart"`).
pub fn parse_forge(content: &str, name: &str) -> Result<Option<ForgeDocument>, ForgeError> {
    parse_forge_with_imports(content, name).map(|opt| opt.map(|pf| pf.document))
}

/// Single-parse entry point that also extracts `<sce:import>` declarations.
/// Returns `None` if the document is a statechart (no `sce:kind` or `sce:kind="statechart"`).
///
/// XSD validation runs as the first step: the document is checked against
/// `schemas/sce-forge.xsd` (W3C wrapper importing the `sce:` namespace
/// extension schema). Any violation — bad enum value, malformed
/// `sce:bit-size`, missing required attribute on `<sce:field>` /
/// `<sce:entry>` / `<sce:import>` — is rejected here with line/column
/// info before any kind-specific parsing runs. If the schema cannot be
/// located (e.g. `sce-build` vendored without the `schemas/` directory),
/// validation is silently skipped — see
/// `xsd_validator::validate_or_skip` for the rationale.
pub fn parse_forge_with_imports(content: &str, name: &str) -> Result<Option<ParsedForge>, ForgeError> {
    crate::forge::xsd_validator::validate_or_skip(content, name)
        .map_err(XmlError::SchemaValidation)?;

    let doc = roxmltree::Document::parse(content)
        .map_err(|e| XmlError::Parse(e.to_string()))?;
    let root = doc.root_element();

    let kind = match detect_kind_from_node(&root)? {
        None => return Ok(None),
        Some(ForgeKind::Statechart) => return Ok(None),
        Some(k) => k,
    };

    if !kind.is_supported() {
        return Err(ValidationError::UnsupportedKind(kind.to_string()).into());
    }

    let imports = parse_imports(&root)?;
    let document = parse_forge_from_node(&root, name, kind)?;
    Ok(Some(ParsedForge { document, imports }))
}

// ── Internal: kind detection from parsed node ──────────────────

fn detect_kind_from_node(root: &roxmltree::Node) -> Result<Option<ForgeKind>, ValidationError> {
    let kind_val = match sce_attr(root, "kind") {
        Some(v) => v,
        None => return Ok(None),
    };
    match ForgeKind::from_attr(&kind_val) {
        Some(kind) => Ok(Some(kind)),
        None => Err(ValidationError::UnsupportedKind(kind_val)),
    }
}

fn parse_forge_from_node(
    root: &roxmltree::Node,
    name: &str,
    kind: ForgeKind,
) -> Result<ForgeDocument, ForgeError> {
    match kind {
        ForgeKind::Transform => Ok(parse_transform(root, name).map(ForgeDocument::Transform)?),
        ForgeKind::Lookup => Ok(parse_lookup(root, name).map(ForgeDocument::Lookup)?),
        ForgeKind::Condition => Ok(parse_condition(root, name).map(ForgeDocument::Condition)?),
        ForgeKind::Codec => Ok(parse_codec(root, name).map(ForgeDocument::Codec)?),
        ForgeKind::Validator => Ok(parse_validator(root, name).map(ForgeDocument::Validator)?),
        ForgeKind::Procedure => Ok(parse_procedure(root, name).map(ForgeDocument::Procedure)?),
        ForgeKind::Filter => Ok(parse_filter(root, name).map(ForgeDocument::Filter)?),
        ForgeKind::Interpolation => Ok(parse_interpolation(root, name).map(ForgeDocument::Interpolation)?),
        ForgeKind::Timer => Ok(parse_timer(root, name).map(ForgeDocument::Timer)?),
        ForgeKind::Observer => Ok(parse_observer(root, name).map(ForgeDocument::Observer)?),
        ForgeKind::Statechart => Err(ValidationError::WrongPipeline {
            kind: ForgeKind::Statechart,
        }.into()),
    }
}

// ── Transform parsing ──────────────────────────────────────────

fn parse_transform(root: &roxmltree::Node, name: &str) -> Result<TransformModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Transform,
            element: "datamodel".into(),
        })?;

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data)?;
        match field.direction {
            Direction::In => inputs.push(field),
            Direction::Out => outputs.push(field),
            Direction::Internal => {
                return Err(ValidationError::InvalidDirection {
                    kind: ForgeKind::Transform,
                    direction: "internal".into(),
                    field: field.id,
                });
            }
        }
    }

    if inputs.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Transform,
            what: "input field".into(),
        });
    }
    if outputs.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Transform,
            what: "output field".into(),
        });
    }

    for out in &outputs {
        if out.expr.is_none() {
            return Err(ValidationError::MissingAttribute {
                element: format!("Transform output field '{}'", out.id),
                attr: "expr".into(),
            });
        }
    }

    Ok(TransformModel {
        name: name.to_string(),
        inputs,
        outputs,
    })
}

// ── Lookup parsing ─────────────────────────────────────────────

fn parse_lookup(root: &roxmltree::Node, name: &str) -> Result<LookupModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Lookup,
            element: "datamodel".into(),
        })?;

    let mut input: Option<ForgeField> = None;
    let mut output: Option<ForgeField> = None;
    let mut entries = Vec::new();
    let mut explicit_default: Option<String> = None;
    let mut on_miss_attr: Option<String> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            input = Some(parse_forge_field(&data)?);
        } else if dir.as_deref() == Some("out") {
            output = Some(parse_forge_field(&data)?);
        } else {
            if let Some(def) = sce_attr(&data, "default") {
                explicit_default = Some(def);
            }
            if let Some(oms) = sce_attr(&data, "on-miss") {
                on_miss_attr = Some(oms);
            }
            entries.extend(parse_sce_entries(&data)?);
        }
    }

    let input = input.ok_or(ValidationError::MissingElement {
        kind: ForgeKind::Lookup,
        element: "input field (sce:direction=\"in\")".into(),
    })?;
    let output = output.ok_or(ValidationError::MissingElement {
        kind: ForgeKind::Lookup,
        element: "output field (sce:direction=\"out\")".into(),
    })?;

    if entries.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Lookup,
            what: "<sce:entry>".into(),
        });
    }

    // Key uniqueness is required by both miss policies: duplicates make the
    // lookup non-deterministic for the colliding key.
    let mut seen_keys = std::collections::BTreeSet::new();
    for entry in &entries {
        if !seen_keys.insert(entry.key.clone()) {
            return Err(ValidationError::DuplicateId {
                kind: ForgeKind::Lookup,
                what: "key".into(),
                id: entry.key.clone(),
            });
        }
    }

    let miss_policy = match on_miss_attr.as_deref() {
        Some("error") => {
            if explicit_default.is_some() {
                return Err(ValidationError::IncompatibleAttributes {
                    element: "Lookup".into(),
                    detail: "sce:on-miss=\"error\" is incompatible with sce:default; \
                             an error policy has no fallback value"
                        .into(),
                });
            }
            MissPolicy::Error
        }
        Some("default") | None => {
            // Absent attribute matches the historical behaviour: fall back to
            // the explicit sce:default value, or the first entry if none.
            let value = explicit_default.unwrap_or_else(|| entries[0].value.clone());
            MissPolicy::Default(value)
        }
        Some(other) => {
            return Err(ValidationError::InvalidAttribute {
                element: "Lookup".into(),
                attr: "sce:on-miss".into(),
                value: other.to_string(),
                expected: "default, error".into(),
            });
        }
    };

    Ok(LookupModel {
        name: name.to_string(),
        input,
        output,
        entries,
        miss_policy,
    })
}

// ── Condition parsing ──────────────────────────────────────────

fn parse_condition(root: &roxmltree::Node, name: &str) -> Result<ConditionModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Condition,
            element: "datamodel".into(),
        })?;

    let mut inputs = Vec::new();
    let mut expr = String::new();

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data)?;
        match field.direction {
            Direction::In => inputs.push(field),
            Direction::Out => {
                if let Some(e) = &field.expr {
                    expr = e.clone();
                } else {
                    return Err(ValidationError::MissingAttribute {
                        element: "Condition output field".into(),
                        attr: "expr".into(),
                    });
                }
            }
            Direction::Internal => {
                return Err(ValidationError::InvalidDirection {
                    kind: ForgeKind::Condition,
                    direction: "internal".into(),
                    field: field.id,
                });
            }
        }
    }

    if inputs.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Condition,
            what: "input field".into(),
        });
    }
    if expr.is_empty() {
        return Err(ValidationError::MissingElement {
            kind: ForgeKind::Condition,
            element: "output field with an 'expr' attribute".into(),
        });
    }

    Ok(ConditionModel {
        name: name.to_string(),
        inputs,
        expr,
    })
}

// ── Codec parsing ──────────────────────────────────────────────

fn parse_codec(root: &roxmltree::Node, name: &str) -> Result<CodecModel, ValidationError> {
    let default_endian = sce_attr(root, "default-endian")
        .and_then(|s| Endian::from_attr(&s))
        .unwrap_or(Endian::Big);

    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Codec,
            element: "datamodel".into(),
        })?;

    let mut fields = Vec::new();
    let mut input_length: Option<u32> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            if let Some(len_str) = sce_attr(&data, "length") {
                input_length = Some(
                    parse_int(&len_str).ok_or_else(|| ValidationError::NumericParse {
                        element: "Codec input".into(),
                        attr: "sce:length".into(),
                        value: len_str,
                        detail: "expected integer".into(),
                    })?,
                );
            }
            continue;
        }

        // Output fields with byte layout (on <data> elements)
        if sce_attr(&data, "byte").is_some() {
            fields.push(parse_codec_field_from_node(&data)?);
        }
    }

    // Also check for <sce:field> elements (used in both standalone and inline codec)
    for child in datamodel.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "field" && child.tag_name().namespace() == Some(SCE_NAMESPACE) {
            fields.push(parse_codec_field_from_node(&child)?);
        }
    }

    if fields.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Codec,
            what: "field with byte layout".into(),
        });
    }

    Ok(CodecModel {
        name: name.to_string(),
        default_endian,
        input_length,
        fields,
    })
}

/// Unified codec field parser — works for both `<data>` and `<sce:field>` elements.
/// Public: also called from SCXMLParser::try_parse_inline_kind() for single-pass extraction.
pub fn parse_codec_field_from_node(node: &roxmltree::Node) -> Result<CodecField, ValidationError> {
    let id = node
        .attribute("id")
        .ok_or(ValidationError::MissingAttribute {
            element: "Codec field".into(),
            attr: "id".into(),
        })?
        .to_string();

    let sce_type_str = sce_attr(node, "type").unwrap_or_else(|| "uint8".to_string());
    let sce_type = SceType::from_attr(&sce_type_str)
        .ok_or_else(|| ValidationError::InvalidAttribute {
            element: format!("field '{id}'"),
            attr: "sce:type".into(),
            value: sce_type_str.clone(),
            expected: "uint8, uint16, uint32, int8, int16, int32, float32, float64, bool, string, bytes".into(),
        })?;

    let byte_offset_str = sce_attr(node, "byte")
        .ok_or_else(|| ValidationError::MissingAttribute {
            element: format!("Codec field '{id}'"),
            attr: "sce:byte".into(),
        })?;
    let byte_offset = parse_int(&byte_offset_str)
        .ok_or_else(|| ValidationError::NumericParse {
            element: format!("field '{id}'"),
            attr: "sce:byte".into(),
            value: byte_offset_str.clone(),
            detail: "expected integer".into(),
        })?;

    let bit_offset = sce_attr(node, "bit-offset").and_then(|s| parse_int(&s));

    let bit_size = {
        let bs = sce_attr(node, "bit-size")
            .ok_or_else(|| ValidationError::MissingAttribute {
                element: format!("Codec field '{id}'"),
                attr: "sce:bit-size".into(),
            })?;
        match bs.as_str() {
            "tail" => BitSize::Tail,
            "length-ref" => BitSize::LengthRef,
            _ => {
                let n = parse_int(&bs)
                    .ok_or_else(|| ValidationError::NumericParse {
                        element: format!("field '{id}'"),
                        attr: "sce:bit-size".into(),
                        value: bs.clone(),
                        detail: "expected integer, 'tail', or 'length-ref'".into(),
                    })?;
                BitSize::Fixed { bits: n }
            }
        }
    };

    let endian = sce_attr(node, "endian").and_then(|s| Endian::from_attr(&s));
    let max_size = sce_attr(node, "max-size").and_then(|s| parse_int(&s));
    let length_field = sce_attr(node, "length-field");

    Ok(CodecField {
        id,
        sce_type,
        byte_offset,
        bit_offset,
        bit_size,
        endian,
        max_size,
        length_field,
    })
}

// ── Validator parsing ──────────────────────────────────────

fn parse_validator(root: &roxmltree::Node, name: &str) -> Result<ValidatorModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Validator,
            element: "datamodel".into(),
        })?;

    let mut inputs = Vec::new();
    let mut ranges = Vec::new();
    let mut rate_of_changes = Vec::new();
    let mut plausibility: Option<String> = None;

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data)?;
        match field.direction {
            Direction::In => {
                // Extract validator rules from sce: attributes on input <data> elements.
                let has_range_min = sce_attr(&data, "range-min");
                let has_range_max = sce_attr(&data, "range-max");
                if has_range_min.is_some() || has_range_max.is_some() {
                    ranges.push(RangeRule {
                        id: field.id.clone(),
                        min: has_range_min,
                        max: has_range_max,
                    });
                }

                if let Some(max_delta) = sce_attr(&data, "max-delta") {
                    let sample_interval_str = sce_attr(&data, "sample-interval")
                        .unwrap_or_else(|| "100ms".to_string());
                    let sample_interval_ms = parse_time_interval(&sample_interval_str)?;
                    rate_of_changes.push(RateOfChangeRule {
                        id: field.id.clone(),
                        max_delta,
                        sample_interval_ms,
                    });
                }

                inputs.push(field);
            }
            Direction::Out => {
                // Extract sce:plausibility expression from output <data> element.
                if let Some(expr) = sce_attr(&data, "plausibility") {
                    if plausibility.is_some() {
                        return Err(ValidationError::SingletonViolation {
                            kind: ForgeKind::Validator,
                            attr: "sce:plausibility".into(),
                        });
                    }
                    plausibility = Some(expr);
                }
            }
            Direction::Internal => {
                return Err(ValidationError::InvalidDirection {
                    kind: ForgeKind::Validator,
                    direction: "internal".into(),
                    field: field.id,
                });
            }
        }
    }

    if inputs.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Validator,
            what: "input field".into(),
        });
    }

    if ranges.is_empty() && rate_of_changes.is_empty() && plausibility.is_none() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Validator,
            what: "rule (sce:range-min/max, sce:max-delta, or sce:plausibility)".into(),
        });
    }

    Ok(ValidatorModel {
        name: name.to_string(),
        inputs,
        rules: ValidatorRules {
            ranges,
            rate_of_changes,
            plausibility,
        },
    })
}

// ── Procedure parsing ──────────────────────────────────────

fn parse_procedure(root: &roxmltree::Node, name: &str) -> Result<ProcedureModel, ValidationError> {
    let initial = root
        .attribute("initial")
        .ok_or(ValidationError::MissingAttribute {
            element: "Procedure <scxml>".into(),
            attr: "initial".into(),
        })?
        .to_string();

    // Parse input and internal fields from <datamodel>
    let mut inputs = Vec::new();
    let mut internals = Vec::new();
    let mut helpers = Vec::new();
    if let Some(datamodel) = find_child(root, "datamodel") {
        for data in data_children(&datamodel) {
            let field = parse_forge_field(&data)?;
            match field.direction {
                Direction::In => inputs.push(field),
                Direction::Internal => internals.push(field),
                Direction::Out => {
                    // Output fields are not used as execute() parameters
                }
            }
        }
        // Collect <sce:helper> DI declarations — user-provided closure members
        // injected via per-language setters. The parser treats them as typed
        // free-function signatures so the expression pipeline can infer return
        // types through enclosing arithmetic / member access. Duplicate names
        // are rejected here so the downstream generator emits clean per-field
        // errors rather than a decipher-the-duplicate-struct-field tailspin.
        for child in datamodel.children().filter(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(SCE_NAMESPACE)
                && n.tag_name().name() == "helper"
        }) {
            let helper = parse_procedure_helper(&child)?;
            if helpers.iter().any(|h: &ProcedureHelper| h.name == helper.name) {
                return Err(ValidationError::DuplicateId {
                    kind: ForgeKind::Procedure,
                    what: "<sce:helper>".into(),
                    id: helper.name,
                });
            }
            helpers.push(helper);
        }
    }

    // Parse <state> and <final> elements
    let mut states = Vec::new();
    let mut state_ids = std::collections::BTreeSet::new();

    for child in root.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        let is_final = tag == "final";
        if tag != "state" && tag != "final" {
            continue;
        }

        let id = child
            .attribute("id")
            .ok_or_else(|| ValidationError::MissingAttribute {
                element: format!("<{tag}>"),
                attr: "id".into(),
            })?
            .to_string();

        if !state_ids.insert(id.clone()) {
            return Err(ValidationError::DuplicateId {
                kind: ForgeKind::Procedure,
                what: "state id".into(),
                id,
            });
        }

        let transitions = if is_final {
            Vec::new()
        } else {
            parse_procedure_transitions(&child)?
        };

        // Parse <onentry> → <send> actions
        let on_entry_sends = parse_procedure_onentry(&child)?;

        // Parse <donedata> on <final> elements
        let done_params = if is_final {
            parse_procedure_donedata(&child)?
        } else {
            Vec::new()
        };

        states.push(ProcedureState {
            id,
            is_final,
            transitions,
            on_entry_sends,
            done_params,
        });
    }

    if states.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Procedure,
            what: "<state> or <final> element".into(),
        });
    }

    // Validate: initial state must exist
    if !state_ids.contains(&initial) {
        return Err(ValidationError::InvalidReference {
            kind: ForgeKind::Procedure,
            name: initial.clone(),
            what: "state".into(),
            available: state_ids.iter().cloned().collect::<Vec<_>>().join(", "),
        });
    }

    // Validate: must have at least one final state
    if !states.iter().any(|s| s.is_final) {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Procedure,
            what: "<final> element".into(),
        });
    }

    // Validate: all transition targets must reference existing states
    for state in &states {
        for tr in &state.transitions {
            if !state_ids.contains(&tr.target) {
                return Err(ValidationError::InvalidReference {
                    kind: ForgeKind::Procedure,
                    name: format!("transition target '{}' in state '{}'", tr.target, state.id),
                    what: "state".into(),
                    available: state_ids.iter().cloned().collect::<Vec<_>>().join(", "),
                });
            }
        }
    }

    // Validate: non-final states must have at least one transition
    for state in &states {
        if !state.is_final && state.transitions.is_empty() {
            return Err(ValidationError::EmptyCollection {
                kind: ForgeKind::Procedure,
                what: format!("<transition> in non-final state '{}'", state.id),
            });
        }
    }

    Ok(ProcedureModel {
        name: name.to_string(),
        inputs,
        internals,
        helpers,
        initial,
        states,
    })
}

/// Validate that `name` matches `[A-Za-z_][A-Za-z0-9_]*` — the common
/// C-family identifier grammar every target language (Rust / C++ / Python /
/// Kotlin / Go) accepts as an unquoted identifier. Helper names flow
/// verbatim into `format!` strings that emit generated source code,
/// per-language error messages, and rename-map keys, so any character
/// outside this grammar (quote, backslash, space, non-ASCII, leading digit,
/// etc.) would break the generator and has no legitimate use case.
fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else { return false };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse `<sce:helper name="..." args="bytes,uint32" returns="bytes"/>`.
/// Zero-arg helpers use `args=""`. Both `args` and `returns` accept the full
/// `SceType::from_attr` vocabulary.
fn parse_procedure_helper(node: &roxmltree::Node) -> Result<ProcedureHelper, ValidationError> {
    let name = node
        .attribute("name")
        .ok_or(ValidationError::MissingAttribute {
            element: "<sce:helper>".into(),
            attr: "name".into(),
        })?
        .to_string();
    if name.is_empty() {
        return Err(ValidationError::EmptyValue {
            element: "<sce:helper>".into(),
            attr: "name".into(),
        });
    }
    if !is_ident(&name) {
        return Err(ValidationError::InvalidAttribute {
            element: "<sce:helper>".into(),
            attr: "name".into(),
            value: name,
            expected: "[A-Za-z_][A-Za-z0-9_]* (valid identifier for 5-language generated source)".into(),
        });
    }
    let args_raw = node.attribute("args").unwrap_or("");
    let mut args = Vec::new();
    for part in args_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let sce_ty = SceType::from_attr(part).ok_or_else(|| ValidationError::InvalidAttribute {
            element: format!("<sce:helper name=\"{name}\">"),
            attr: "args".into(),
            value: part.to_string(),
            expected: "valid sce:type".into(),
        })?;
        args.push(sce_ty);
    }
    let returns_raw = node
        .attribute("returns")
        .ok_or_else(|| ValidationError::MissingAttribute {
            element: format!("<sce:helper name=\"{name}\">"),
            attr: "returns".into(),
        })?;
    let returns = SceType::from_attr(returns_raw).ok_or_else(|| ValidationError::InvalidAttribute {
        element: format!("<sce:helper name=\"{name}\">"),
        attr: "returns".into(),
        value: returns_raw.to_string(),
        expected: "valid sce:type".into(),
    })?;
    Ok(ProcedureHelper {
        name,
        args,
        returns,
    })
}

/// Parse <transition> children of a procedure state.
/// Level 1: target + cond only.
/// Level 2: + event + <assign> children.
fn parse_procedure_transitions(state: &roxmltree::Node) -> Result<Vec<ProcedureTransition>, ValidationError> {
    let mut transitions = Vec::new();

    for child in state.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "transition" {
            continue;
        }

        let target = child
            .attribute("target")
            .ok_or_else(|| ValidationError::MissingAttribute {
                element: format!(
                    "<transition> in state '{}'",
                    state.attribute("id").unwrap_or("?")
                ),
                attr: "target".into(),
            })?
            .to_string();

        let cond = child.attribute("cond").map(|s| s.to_string());
        let event = child.attribute("event").map(|s| s.to_string());

        // Parse <assign> children within the transition (Level 2)
        let assigns = parse_procedure_assigns(&child)?;

        transitions.push(ProcedureTransition {
            target,
            cond,
            event,
            assigns,
        });
    }

    Ok(transitions)
}

/// Parse <assign> children within a <transition> element.
fn parse_procedure_assigns(transition: &roxmltree::Node) -> Result<Vec<ProcedureAssign>, ValidationError> {
    let mut assigns = Vec::new();
    for child in transition.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "assign" {
            continue;
        }
        let location = child
            .attribute("location")
            .ok_or(ValidationError::MissingAttribute {
                element: "<assign>".into(),
                attr: "location".into(),
            })?
            .to_string();
        let expr = child
            .attribute("expr")
            .ok_or(ValidationError::MissingAttribute {
                element: "<assign>".into(),
                attr: "expr".into(),
            })?
            .to_string();
        assigns.push(ProcedureAssign { location, expr });
    }
    Ok(assigns)
}

/// Parse <onentry> → <send> actions within a procedure state.
fn parse_procedure_onentry(state: &roxmltree::Node) -> Result<Vec<ProcedureSendAction>, ValidationError> {
    let mut sends = Vec::new();
    for onentry in state.children().filter(|n| n.is_element() && n.tag_name().name() == "onentry") {
        for child in onentry.children().filter(|n| n.is_element()) {
            if child.tag_name().name() != "send" {
                continue;
            }
            let service = sce_attr(&child, "service")
                .ok_or(ValidationError::MissingAttribute {
                    element: "<send> in procedure <onentry>".into(),
                    attr: "sce:service".into(),
                })?;
            let subfunc = sce_attr(&child, "subfunc");
            let addr = sce_attr(&child, "addr");
            let payload = sce_attr(&child, "payload");
            sends.push(ProcedureSendAction {
                service,
                subfunc,
                addr,
                payload,
            });
        }
    }
    Ok(sends)
}

/// Parse <donedata> → <param> children within a <final> element.
fn parse_procedure_donedata(final_elem: &roxmltree::Node) -> Result<Vec<ProcedureDoneParam>, ValidationError> {
    let mut params = Vec::new();
    for donedata in final_elem
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "donedata")
    {
        for child in donedata.children().filter(|n| n.is_element()) {
            if child.tag_name().name() != "param" {
                continue;
            }
            let param_name = child
                .attribute("name")
                .ok_or(ValidationError::MissingAttribute {
                    element: "<param> in <donedata>".into(),
                    attr: "name".into(),
                })?
                .to_string();
            let expr = child
                .attribute("expr")
                .ok_or(ValidationError::MissingAttribute {
                    element: "<param> in <donedata>".into(),
                    attr: "expr".into(),
                })?
                .to_string();
            params.push(ProcedureDoneParam {
                name: param_name,
                expr,
            });
        }
    }
    Ok(params)
}

/// Parse time interval like "100ms" or "1s" into milliseconds.
fn parse_time_interval(s: &str) -> Result<u32, ValidationError> {
    let s = s.trim();
    if let Some(ms_str) = s.strip_suffix("ms") {
        ms_str
            .parse::<u32>()
            .map_err(|_| ValidationError::NumericParse {
                element: "time interval".into(),
                attr: "value".into(),
                value: s.to_string(),
                detail: "expected integer with 'ms' suffix".into(),
            })
    } else if let Some(s_str) = s.strip_suffix('s') {
        s_str
            .parse::<u32>()
            .map(|secs| secs * 1000)
            .map_err(|_| ValidationError::NumericParse {
                element: "time interval".into(),
                attr: "value".into(),
                value: s.to_string(),
                detail: "expected integer with 's' suffix".into(),
            })
    } else {
        Err(ValidationError::NumericParse {
            element: "time interval".into(),
            attr: "value".into(),
            value: s.to_string(),
            detail: "must end with 'ms' or 's'".into(),
        })
    }
}

// ── Filter parsing ────────────────────────────────────────────

fn parse_filter(root: &roxmltree::Node, name: &str) -> Result<FilterModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Filter,
            element: "datamodel".into(),
        })?;

    let mut input: Option<ForgeField> = None;
    let mut output: Option<ForgeField> = None;
    let mut filter_type: Option<FilterType> = None;
    let mut window: Option<u32> = None;
    let mut alpha: Option<f64> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");
        match dir.as_deref() {
            Some("in") => {
                input = Some(parse_forge_field(&data)?);
            }
            Some("out") => {
                output = Some(parse_forge_field(&data)?);

                let ft_str = sce_attr(&data, "filter")
                    .ok_or(ValidationError::MissingAttribute {
                        element: "Filter output".into(),
                        attr: "sce:filter".into(),
                    })?;
                filter_type = Some(
                    FilterType::from_attr(&ft_str)
                        .ok_or_else(|| ValidationError::InvalidAttribute {
                            element: "Filter output".into(),
                            attr: "sce:filter".into(),
                            value: ft_str.clone(),
                            expected: "moving-average, low-pass, debounce".into(),
                        })?,
                );

                window = sce_attr(&data, "window").and_then(|s| s.parse::<u32>().ok());
                alpha = sce_attr(&data, "alpha").and_then(|s| s.parse::<f64>().ok());
            }
            _ => {
                return Err(ValidationError::InvalidDirection {
                    kind: ForgeKind::Filter,
                    direction: dir.unwrap_or_default(),
                    field: String::new(),
                });
            }
        }
    }

    let input = input.ok_or(ValidationError::MissingElement {
        kind: ForgeKind::Filter,
        element: "input field (sce:direction=\"in\")".into(),
    })?;
    let output = output.ok_or(ValidationError::MissingElement {
        kind: ForgeKind::Filter,
        element: "output field (sce:direction=\"out\")".into(),
    })?;
    let filter_type = filter_type.ok_or(ValidationError::MissingAttribute {
        element: "Filter output".into(),
        attr: "sce:filter".into(),
    })?;

    // Validate required parameters per filter type
    match filter_type {
        FilterType::MovingAverage => {
            if window.is_none() {
                return Err(ValidationError::MissingAttribute {
                    element: "Moving-average filter".into(),
                    attr: "sce:window".into(),
                });
            }
        }
        FilterType::LowPass => {
            if alpha.is_none() {
                return Err(ValidationError::MissingAttribute {
                    element: "Low-pass filter".into(),
                    attr: "sce:alpha".into(),
                });
            }
        }
        FilterType::Debounce => {
            if window.is_none() {
                return Err(ValidationError::MissingAttribute {
                    element: "Debounce filter".into(),
                    attr: "sce:window".into(),
                });
            }
        }
    }

    Ok(FilterModel {
        name: name.to_string(),
        input,
        output,
        filter_type,
        window,
        alpha,
    })
}

// ── Interpolation parsing ─────────────────────────────────────

fn parse_interpolation(root: &roxmltree::Node, name: &str) -> Result<InterpolationModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Interpolation,
            element: "datamodel".into(),
        })?;

    let mut inputs = Vec::new();
    let mut output: Option<ForgeField> = None;
    let mut method: Option<InterpolationMethod> = None;
    let mut out_of_bounds = OutOfBounds::default();
    let mut axes = Vec::new();
    let mut values = Vec::new();

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");
        match dir.as_deref() {
            Some("in") => {
                inputs.push(parse_forge_field(&data)?);
            }
            Some("out") => {
                output = Some(parse_forge_field(&data)?);

                let method_str = sce_attr(&data, "interpolation")
                    .ok_or(ValidationError::MissingAttribute {
                        element: "Interpolation output".into(),
                        attr: "sce:interpolation".into(),
                    })?;
                method = Some(
                    InterpolationMethod::from_attr(&method_str)
                        .ok_or_else(|| ValidationError::InvalidAttribute {
                            element: "Interpolation output".into(),
                            attr: "sce:interpolation".into(),
                            value: method_str.clone(),
                            expected: "linear, bilinear".into(),
                        })?,
                );

                if let Some(oob_str) = sce_attr(&data, "out-of-bounds") {
                    out_of_bounds = OutOfBounds::from_attr(&oob_str)
                        .ok_or_else(|| ValidationError::InvalidAttribute {
                            element: "Interpolation output".into(),
                            attr: "sce:out-of-bounds".into(),
                            value: oob_str.clone(),
                            expected: "clamp, extrapolate, error".into(),
                        })?;
                }

                // Parse sce:axis-{input_id} attributes
                for inp in &inputs {
                    let axis_attr = format!("axis-{}", inp.id);
                    if let Some(bp_str) = sce_attr(&data, &axis_attr) {
                        let breakpoints: Result<Vec<f64>, _> = bp_str
                            .split_whitespace()
                            .map(|s| s.parse::<f64>())
                            .collect();
                        let breakpoints = breakpoints
                            .map_err(|e| ValidationError::NumericParse {
                                element: format!("Interpolation axis-{}", inp.id),
                                attr: format!("sce:axis-{}", inp.id),
                                value: bp_str.clone(),
                                detail: e.to_string(),
                            })?;
                        axes.push(InterpolationAxis {
                            input_id: inp.id.clone(),
                            breakpoints,
                        });
                    }
                }

                // Parse table values from text content
                let text = data.text().unwrap_or("").trim().to_string();
                if !text.is_empty() {
                    values = text
                        .split_whitespace()
                        .map(|s| s.parse::<f64>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| ValidationError::NumericParse {
                            element: "Interpolation output".into(),
                            attr: "table values".into(),
                            value: text.clone(),
                            detail: e.to_string(),
                        })?;
                }
            }
            _ => {
                return Err(ValidationError::InvalidDirection {
                    kind: ForgeKind::Interpolation,
                    direction: dir.unwrap_or_default(),
                    field: String::new(),
                });
            }
        }
    }

    let output = output.ok_or(ValidationError::MissingElement {
        kind: ForgeKind::Interpolation,
        element: "output field (sce:direction=\"out\")".into(),
    })?;
    let method = method.ok_or(ValidationError::MissingAttribute {
        element: "Interpolation output".into(),
        attr: "sce:interpolation".into(),
    })?;

    if inputs.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Interpolation,
            what: "input field".into(),
        });
    }
    if axes.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Interpolation,
            what: "sce:axis-* attribute".into(),
        });
    }
    if values.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Interpolation,
            what: "table values in the output element text".into(),
        });
    }

    // Validate axis count matches method
    match method {
        InterpolationMethod::Linear => {
            if axes.len() != 1 {
                return Err(ValidationError::CountMismatch {
                    kind: ForgeKind::Interpolation,
                    detail: "linear: requires exactly 1 axis".into(),
                });
            }
            if values.len() != axes[0].breakpoints.len() {
                return Err(ValidationError::CountMismatch {
                    kind: ForgeKind::Interpolation,
                    detail: format!(
                        "linear: value count ({}) must match axis breakpoints ({})",
                        values.len(),
                        axes[0].breakpoints.len()
                    ),
                });
            }
        }
        InterpolationMethod::Bilinear => {
            if axes.len() != 2 {
                return Err(ValidationError::CountMismatch {
                    kind: ForgeKind::Interpolation,
                    detail: "bilinear: requires exactly 2 axes".into(),
                });
            }
            let expected = axes[0].breakpoints.len() * axes[1].breakpoints.len();
            if values.len() != expected {
                return Err(ValidationError::CountMismatch {
                    kind: ForgeKind::Interpolation,
                    detail: format!(
                        "bilinear: value count ({}) must equal rows({}) x cols({}) = {}",
                        values.len(),
                        axes[0].breakpoints.len(),
                        axes[1].breakpoints.len(),
                        expected
                    ),
                });
            }
        }
    }

    Ok(InterpolationModel {
        name: name.to_string(),
        inputs,
        output,
        method,
        out_of_bounds,
        axes,
        values,
    })
}

// ── Timer parsing ─────────────────────────────────────────────

fn parse_timer(root: &roxmltree::Node, name: &str) -> Result<TimerModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Timer,
            element: "datamodel".into(),
        })?;

    let mut timers = Vec::new();

    for data in data_children(&datamodel) {
        let timer_str = match sce_attr(&data, "timer") {
            Some(s) => s,
            None => continue,
        };

        let id = data
            .attribute("id")
            .ok_or(ValidationError::MissingAttribute {
                element: "Timer <data>".into(),
                attr: "id".into(),
            })?
            .to_string();

        let timer_type = TimerType::from_attr(&timer_str)
            .ok_or_else(|| ValidationError::InvalidAttribute {
                element: format!("Timer '{id}'"),
                attr: "sce:timer".into(),
                value: timer_str.clone(),
                expected: "periodic, timeout, delayed".into(),
            })?;

        let time_ms = match timer_type {
            TimerType::Periodic => {
                let s = sce_attr(&data, "interval")
                    .ok_or_else(|| ValidationError::MissingAttribute {
                        element: format!("Periodic timer '{id}'"),
                        attr: "sce:interval".into(),
                    })?;
                s.parse::<u32>()
                    .map_err(|_| ValidationError::NumericParse {
                        element: format!("timer '{id}'"),
                        attr: "sce:interval".into(),
                        value: s.clone(),
                        detail: "expected integer".into(),
                    })?
            }
            TimerType::Timeout => {
                let s = sce_attr(&data, "duration")
                    .ok_or_else(|| ValidationError::MissingAttribute {
                        element: format!("Timeout timer '{id}'"),
                        attr: "sce:duration".into(),
                    })?;
                s.parse::<u32>()
                    .map_err(|_| ValidationError::NumericParse {
                        element: format!("timer '{id}'"),
                        attr: "sce:duration".into(),
                        value: s.clone(),
                        detail: "expected integer".into(),
                    })?
            }
            TimerType::Delayed => {
                let s = sce_attr(&data, "delay")
                    .ok_or_else(|| ValidationError::MissingAttribute {
                        element: format!("Delayed timer '{id}'"),
                        attr: "sce:delay".into(),
                    })?;
                s.parse::<u32>()
                    .map_err(|_| ValidationError::NumericParse {
                        element: format!("timer '{id}'"),
                        attr: "sce:delay".into(),
                        value: s.clone(),
                        detail: "expected integer".into(),
                    })?
            }
        };

        let event = sce_attr(&data, "event");
        let on_timeout = sce_attr(&data, "on-timeout");

        if event.is_none() && on_timeout.is_none() {
            return Err(ValidationError::RequireEither {
                element: format!("Timer '{id}'"),
                alternatives: vec!["sce:event".into(), "sce:on-timeout".into()],
            });
        }

        timers.push(TimerEntry {
            id,
            timer_type,
            time_ms,
            event,
            on_timeout,
        });
    }

    if timers.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Timer,
            what: "<data> with 'sce:timer' attribute".into(),
        });
    }

    Ok(TimerModel {
        name: name.to_string(),
        timers,
    })
}

// ── Observer parsing ──────────────────────────────────────────

fn parse_observer(root: &roxmltree::Node, name: &str) -> Result<ObserverModel, ValidationError> {
    let datamodel = find_child(root, "datamodel")
        .ok_or(ValidationError::MissingElement {
            kind: ForgeKind::Observer,
            element: "datamodel".into(),
        })?;

    let event_domain = sce_attr(root, "event-domain");

    let mut inputs = Vec::new();
    let mut monitors = Vec::new();

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            inputs.push(parse_forge_field(&data)?);
            continue;
        }

        // Monitor definitions have sce:monitor attribute
        if let Some(monitor_type) = sce_attr(&data, "monitor") {
            if monitor_type != "threshold" {
                return Err(ValidationError::InvalidAttribute {
                    element: "Observer monitor".into(),
                    attr: "sce:monitor".into(),
                    value: monitor_type,
                    expected: "threshold".into(),
                });
            }

            let id = data
                .attribute("id")
                .ok_or(ValidationError::MissingAttribute {
                    element: "Observer monitor <data>".into(),
                    attr: "id".into(),
                })?
                .to_string();

            let enter_expr = sce_attr(&data, "enter")
                .ok_or_else(|| ValidationError::MissingAttribute {
                    element: format!("Monitor '{id}'"),
                    attr: "sce:enter".into(),
                })?;

            let leave_expr = sce_attr(&data, "leave");

            let on_enter = sce_attr(&data, "on-enter")
                .ok_or_else(|| ValidationError::MissingAttribute {
                    element: format!("Monitor '{id}'"),
                    attr: "sce:on-enter".into(),
                })?;

            let on_leave = sce_attr(&data, "on-leave");

            monitors.push(ThresholdMonitor {
                id,
                enter_expr,
                leave_expr,
                on_enter,
                on_leave,
            });
        }
    }

    if inputs.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Observer,
            what: "input field".into(),
        });
    }
    if monitors.is_empty() {
        return Err(ValidationError::EmptyCollection {
            kind: ForgeKind::Observer,
            what: "monitor definition".into(),
        });
    }

    Ok(ObserverModel {
        name: name.to_string(),
        inputs,
        monitors,
        event_domain,
    })
}

// ── Import parsing ────────────────────────────────────────────

/// Parse `<sce:import>` children from the `<scxml>` root element.
///
/// ```xml
/// <sce:import src="can_frame.scxml" kind="codec" as="frame"/>
/// ```
///
/// - `src` (required): relative path to the imported SCXML file.
/// - `kind` (required): the forge kind of the imported document.
/// - `as` (required): alias used in expressions.
fn parse_imports(root: &roxmltree::Node) -> Result<Vec<ForgeImport>, ValidationError> {
    let mut imports = Vec::new();
    let mut aliases = std::collections::BTreeSet::new();

    for child in root.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "import"
            || child.tag_name().namespace() != Some(SCE_NAMESPACE)
        {
            continue;
        }

        let src = child
            .attribute("src")
            .ok_or(ValidationError::MissingAttribute {
                element: "<sce:import>".into(),
                attr: "src".into(),
            })?
            .to_string();

        let kind_str = child
            .attribute("kind")
            .ok_or(ValidationError::MissingAttribute {
                element: "<sce:import>".into(),
                attr: "kind".into(),
            })?;
        let kind = ForgeKind::from_attr(kind_str)
            .ok_or_else(|| ValidationError::UnsupportedKind(kind_str.to_string()))?;
        if !kind.is_supported() {
            return Err(ValidationError::UnsupportedKind(kind.to_string()));
        }
        if kind == ForgeKind::Statechart {
            return Err(ValidationError::WrongPipeline {
                kind: ForgeKind::Statechart,
            });
        }

        let alias = child
            .attribute("as")
            .ok_or(ValidationError::MissingAttribute {
                element: "<sce:import>".into(),
                attr: "as".into(),
            })?
            .to_string();

        if !aliases.insert(alias.clone()) {
            return Err(ValidationError::DuplicateId {
                kind,
                what: "alias".into(),
                id: alias,
            });
        }

        imports.push(ForgeImport { src, kind, alias });
    }

    Ok(imports)
}

/// Extract only the import list from a forge SCXML (lightweight — no model parse).
/// Used by the manifest scanner to build dependency graphs.
pub fn parse_imports_only(content: &str) -> Result<Vec<ForgeImport>, ForgeError> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| XmlError::Parse(e.to_string()))?;
    let root = doc.root_element();
    Ok(parse_imports(&root)?)
}

// ── Shared helpers ─────────────────────────────────────────────

/// Read an `sce:xxx` attribute from a node (namespace-qualified).
fn sce_attr(node: &roxmltree::Node, local_name: &str) -> Option<String> {
    node.attribute((SCE_NAMESPACE, local_name))
        .map(|s| s.to_string())
}

/// Parse `<sce:entry key="..." value="..."/>` children from a node.
fn parse_sce_entries(node: &roxmltree::Node) -> Result<Vec<LookupEntry>, ValidationError> {
    let mut entries = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "entry" && child.tag_name().namespace() == Some(SCE_NAMESPACE) {
            let key = child
                .attribute("key")
                .ok_or(ValidationError::MissingAttribute {
                    element: "<sce:entry>".into(),
                    attr: "key".into(),
                })?
                .to_string();
            let value = child
                .attribute("value")
                .ok_or(ValidationError::MissingAttribute {
                    element: "<sce:entry>".into(),
                    attr: "value".into(),
                })?
                .to_string();
            entries.push(LookupEntry { key, value });
        }
    }
    Ok(entries)
}

/// Parse a typed forge field from a <data> element.
fn parse_forge_field(data: &roxmltree::Node) -> Result<ForgeField, ValidationError> {
    let id = data
        .attribute("id")
        .ok_or(ValidationError::MissingAttribute {
            element: "Forge <data> field".into(),
            attr: "id".into(),
        })?
        .to_string();

    let type_str = sce_attr(data, "type")
        .ok_or_else(|| ValidationError::MissingAttribute {
            element: format!("Field '{id}'"),
            attr: "sce:type".into(),
        })?;
    let sce_type = SceType::from_attr(&type_str)
        .ok_or_else(|| ValidationError::InvalidAttribute {
            element: format!("field '{id}'"),
            attr: "sce:type".into(),
            value: type_str.clone(),
            expected: "uint8, uint16, uint32, int8, int16, int32, float32, float64, bool, string, bytes".into(),
        })?;

    let dir_str = sce_attr(data, "direction")
        .ok_or_else(|| ValidationError::MissingAttribute {
            element: format!("Field '{id}'"),
            attr: "sce:direction".into(),
        })?;
    let direction = Direction::from_attr(&dir_str)
        .ok_or_else(|| ValidationError::InvalidAttribute {
            element: format!("field '{id}'"),
            attr: "sce:direction".into(),
            value: dir_str.clone(),
            expected: "in, out, internal".into(),
        })?;

    let expr = data.attribute("expr").map(|s| s.to_string());
    let unit = sce_attr(data, "unit");

    Ok(ForgeField {
        id,
        sce_type,
        direction,
        expr,
        unit,
    })
}

/// Find a direct child element by local name.
fn find_child<'a>(node: &'a roxmltree::Node, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

/// Iterate over <data> children of a <datamodel> element.
fn data_children<'a>(
    datamodel: &'a roxmltree::Node,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    datamodel
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "data")
}

/// Parse an integer from a string (supports 0x hex prefix).
fn parse_int(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}
