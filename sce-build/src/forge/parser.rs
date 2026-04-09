// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Forge parser — extracts kind-specific models from Extended SCXML.
//
// Reads `sce:kind` on <scxml> root and dispatches to kind-specific parsing.
// Also handles inline kinds on <data> elements within statechart documents.

use crate::forge::model::*;

/// Detect the `sce:kind` attribute on the <scxml> root element.
/// Returns `None` if no `sce:kind` is present (defaults to statechart).
pub fn detect_kind(content: &str) -> Result<Option<ForgeKind>, String> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| format!("XML parse error: {e}"))?;
    let root = doc.root_element();
    detect_kind_from_node(&root)
}

/// Single-parse entry point: detect kind and parse forge document in one pass.
/// Returns `None` if the document is a statechart (no `sce:kind` or `sce:kind="statechart"`).
pub fn parse_forge(content: &str, name: &str) -> Result<Option<ForgeDocument>, String> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| format!("XML parse error: {e}"))?;
    let root = doc.root_element();

    let kind = match detect_kind_from_node(&root)? {
        None => return Ok(None),
        Some(ForgeKind::Statechart) => return Ok(None),
        Some(k) => k,
    };

    if !kind.is_supported() {
        return Err(format!("Kind '{kind}' is not yet supported"));
    }

    parse_forge_from_node(&root, name, kind).map(Some)
}

// ── Internal: kind detection from parsed node ──────────────────

fn detect_kind_from_node(root: &roxmltree::Node) -> Result<Option<ForgeKind>, String> {
    let kind_val = match sce_attr(root, "kind") {
        Some(v) => v,
        None => return Ok(None),
    };
    match ForgeKind::from_attr(&kind_val) {
        Some(kind) => Ok(Some(kind)),
        None => Err(format!("Unknown sce:kind value: '{kind_val}'")),
    }
}

fn parse_forge_from_node(
    root: &roxmltree::Node,
    name: &str,
    kind: ForgeKind,
) -> Result<ForgeDocument, String> {
    match kind {
        ForgeKind::Transform => parse_transform(root, name).map(ForgeDocument::Transform),
        ForgeKind::Lookup => parse_lookup(root, name).map(ForgeDocument::Lookup),
        ForgeKind::Condition => parse_condition(root, name).map(ForgeDocument::Condition),
        ForgeKind::Codec => parse_codec(root, name).map(ForgeDocument::Codec),
        ForgeKind::Validator => parse_validator(root, name).map(ForgeDocument::Validator),
        ForgeKind::Procedure => parse_procedure(root, name).map(ForgeDocument::Procedure),
        ForgeKind::Statechart => Err("Statechart kind uses the standard pipeline".to_string()),
        other => Err(format!("Kind '{other}' is not yet supported")),
    }
}

// ── Transform parsing ──────────────────────────────────────────

fn parse_transform(root: &roxmltree::Node, name: &str) -> Result<TransformModel, String> {
    let datamodel = find_child(root, "datamodel")
        .ok_or("Transform kind requires a <datamodel> element")?;

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data)?;
        match field.direction {
            Direction::In => inputs.push(field),
            Direction::Out => outputs.push(field),
            Direction::Internal => {
                return Err(format!(
                    "Transform kind does not support 'internal' direction (field '{}')",
                    field.id
                ));
            }
        }
    }

    if inputs.is_empty() {
        return Err("Transform kind requires at least one input field".to_string());
    }
    if outputs.is_empty() {
        return Err("Transform kind requires at least one output field".to_string());
    }

    for out in &outputs {
        if out.expr.is_none() {
            return Err(format!(
                "Transform output field '{}' must have an 'expr' attribute",
                out.id
            ));
        }
    }

    Ok(TransformModel {
        name: name.to_string(),
        inputs,
        outputs,
    })
}

// ── Lookup parsing ─────────────────────────────────────────────

fn parse_lookup(root: &roxmltree::Node, name: &str) -> Result<LookupModel, String> {
    let datamodel = find_child(root, "datamodel")
        .ok_or("Lookup kind requires a <datamodel> element")?;

    let mut input: Option<ForgeField> = None;
    let mut output: Option<ForgeField> = None;
    let mut entries = Vec::new();
    let mut default_value = String::new();

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            input = Some(parse_forge_field(&data)?);
        } else if dir.as_deref() == Some("out") {
            output = Some(parse_forge_field(&data)?);
        } else {
            if let Some(def) = sce_attr(&data, "default") {
                default_value = def;
            }
            entries.extend(parse_sce_entries(&data)?);
        }
    }

    let input = input.ok_or("Lookup kind requires an input field (sce:direction=\"in\")")?;
    let output = output.ok_or("Lookup kind requires an output field (sce:direction=\"out\")")?;

    if entries.is_empty() {
        return Err("Lookup kind requires at least one <sce:entry>".to_string());
    }

    if default_value.is_empty() {
        default_value = entries[0].value.clone();
    }

    Ok(LookupModel {
        name: name.to_string(),
        input,
        output,
        entries,
        default_value,
    })
}

// ── Condition parsing ──────────────────────────────────────────

fn parse_condition(root: &roxmltree::Node, name: &str) -> Result<ConditionModel, String> {
    let datamodel = find_child(root, "datamodel")
        .ok_or("Condition kind requires a <datamodel> element")?;

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
                    return Err("Condition output field must have an 'expr' attribute".to_string());
                }
            }
            Direction::Internal => {
                return Err("Condition kind does not support 'internal' direction".to_string());
            }
        }
    }

    if inputs.is_empty() {
        return Err("Condition kind requires at least one input field".to_string());
    }
    if expr.is_empty() {
        return Err(
            "Condition kind requires an output field with an 'expr' attribute".to_string(),
        );
    }

    Ok(ConditionModel {
        name: name.to_string(),
        inputs,
        expr,
    })
}

// ── Codec parsing ──────────────────────────────────────────────

fn parse_codec(root: &roxmltree::Node, name: &str) -> Result<CodecModel, String> {
    let default_endian = sce_attr(root, "default-endian")
        .and_then(|s| Endian::from_attr(&s))
        .unwrap_or(Endian::Big);

    let datamodel = find_child(root, "datamodel")
        .ok_or("Codec kind requires a <datamodel> element")?;

    let mut fields = Vec::new();
    let mut input_length: Option<u32> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            if let Some(len_str) = sce_attr(&data, "length") {
                input_length = Some(
                    parse_int(&len_str)
                        .ok_or_else(|| format!("Invalid sce:length value: '{len_str}'"))?,
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
        return Err("Codec kind requires at least one field with byte layout".to_string());
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
pub fn parse_codec_field_from_node(node: &roxmltree::Node) -> Result<CodecField, String> {
    let id = node
        .attribute("id")
        .ok_or("Codec field must have an 'id' attribute")?
        .to_string();

    let sce_type_str = sce_attr(node, "type").unwrap_or_else(|| "uint8".to_string());
    let sce_type = SceType::from_attr(&sce_type_str)
        .ok_or_else(|| format!("Unknown sce:type '{sce_type_str}' on field '{id}'"))?;

    let byte_offset_str = sce_attr(node, "byte")
        .ok_or_else(|| format!("Codec field '{id}' must have 'sce:byte' attribute"))?;
    let byte_offset = parse_int(&byte_offset_str)
        .ok_or_else(|| format!("Invalid sce:byte value '{byte_offset_str}' on field '{id}'"))?;

    let bit_offset = sce_attr(node, "bit-offset").and_then(|s| parse_int(&s));

    let bit_size = {
        let bs = sce_attr(node, "bit-size")
            .ok_or_else(|| format!("Codec field '{id}' must have 'sce:bit-size'"))?;
        match bs.as_str() {
            "tail" => BitSize::Tail,
            "length-ref" => BitSize::LengthRef,
            _ => {
                let n = parse_int(&bs)
                    .ok_or_else(|| format!("Invalid sce:bit-size '{bs}' on field '{id}'"))?;
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

fn parse_validator(root: &roxmltree::Node, name: &str) -> Result<ValidatorModel, String> {
    let datamodel = find_child(root, "datamodel")
        .ok_or("Validator kind requires a <datamodel> element")?;

    let mut inputs = Vec::new();

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data)?;
        match field.direction {
            Direction::In => inputs.push(field),
            Direction::Out => {
                // Output field (bool valid) is implicit in the generated ValidationResult
            }
            Direction::Internal => {
                return Err(format!(
                    "Validator kind does not support 'internal' direction (field '{}')",
                    field.id
                ));
            }
        }
    }

    if inputs.is_empty() {
        return Err("Validator kind requires at least one input field".to_string());
    }

    // Parse <sce:rules> element
    let rules_elem = root
        .children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "rules"
                && n.tag_name().namespace() == Some(SCE_NAMESPACE)
        })
        .ok_or("Validator kind requires a <sce:rules> element")?;

    let mut ranges = Vec::new();
    let mut rate_of_changes = Vec::new();
    let mut plausibility: Option<String> = None;

    for child in rules_elem.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        match child.tag_name().name() {
            "range" => {
                ranges.push(parse_range_rule(&child)?);
            }
            "rate-of-change" => {
                rate_of_changes.push(parse_rate_of_change_rule(&child)?);
            }
            "plausibility" => {
                if plausibility.is_some() {
                    return Err(
                        "Only one <sce:plausibility> element allowed".to_string(),
                    );
                }
                plausibility = Some(
                    child
                        .attribute("expr")
                        .ok_or("<sce:plausibility> must have 'expr' attribute")?
                        .to_string(),
                );
            }
            _ => {}
        }
    }

    if ranges.is_empty() && rate_of_changes.is_empty() && plausibility.is_none() {
        return Err(
            "Validator kind requires at least one rule (range, rate-of-change, or plausibility)"
                .to_string(),
        );
    }

    // Validate that rule IDs reference existing input fields
    let input_ids: Vec<&str> = inputs.iter().map(|f| f.id.as_str()).collect();
    for r in &ranges {
        if !input_ids.contains(&r.id.as_str()) {
            return Err(format!(
                "<sce:range id='{}'> does not match any input field (available: {})",
                r.id,
                input_ids.join(", ")
            ));
        }
    }
    for roc in &rate_of_changes {
        if !input_ids.contains(&roc.id.as_str()) {
            return Err(format!(
                "<sce:rate-of-change id='{}'> does not match any input field (available: {})",
                roc.id,
                input_ids.join(", ")
            ));
        }
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

fn parse_procedure(root: &roxmltree::Node, name: &str) -> Result<ProcedureModel, String> {
    let initial = root
        .attribute("initial")
        .ok_or("Procedure kind requires 'initial' attribute on <scxml>")?
        .to_string();

    // Parse input fields from <datamodel>
    let mut inputs = Vec::new();
    if let Some(datamodel) = find_child(root, "datamodel") {
        for data in data_children(&datamodel) {
            let field = parse_forge_field(&data)?;
            match field.direction {
                Direction::In => inputs.push(field),
                Direction::Out | Direction::Internal => {
                    // Output/internal fields are not used as execute() parameters
                }
            }
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
            .ok_or_else(|| format!("<{tag}> element must have an 'id' attribute"))?
            .to_string();

        if !state_ids.insert(id.clone()) {
            return Err(format!("Duplicate state id: '{id}'"));
        }

        let transitions = if is_final {
            Vec::new()
        } else {
            parse_procedure_transitions(&child)?
        };

        states.push(ProcedureState {
            id,
            is_final,
            transitions,
        });
    }

    if states.is_empty() {
        return Err("Procedure kind requires at least one <state> or <final> element".to_string());
    }

    // Validate: initial state must exist
    if !state_ids.contains(&initial) {
        return Err(format!(
            "Initial state '{initial}' does not match any state (available: {})",
            state_ids.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }

    // Validate: must have at least one final state
    if !states.iter().any(|s| s.is_final) {
        return Err("Procedure kind requires at least one <final> element".to_string());
    }

    // Validate: all transition targets must reference existing states
    for state in &states {
        for tr in &state.transitions {
            if !state_ids.contains(&tr.target) {
                return Err(format!(
                    "Transition target '{}' in state '{}' does not match any state (available: {})",
                    tr.target,
                    state.id,
                    state_ids.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }
    }

    // Validate: non-final states must have at least one transition
    for state in &states {
        if !state.is_final && state.transitions.is_empty() {
            return Err(format!(
                "Non-final state '{}' must have at least one <transition>",
                state.id
            ));
        }
    }

    Ok(ProcedureModel {
        name: name.to_string(),
        inputs,
        initial,
        states,
    })
}

/// Parse <transition> children of a procedure state.
fn parse_procedure_transitions(state: &roxmltree::Node) -> Result<Vec<ProcedureTransition>, String> {
    let mut transitions = Vec::new();

    for child in state.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "transition" {
            continue;
        }

        let target = child
            .attribute("target")
            .ok_or_else(|| {
                format!(
                    "<transition> in state '{}' must have a 'target' attribute",
                    state.attribute("id").unwrap_or("?")
                )
            })?
            .to_string();

        let cond = child.attribute("cond").map(|s| s.to_string());

        transitions.push(ProcedureTransition { target, cond });
    }

    Ok(transitions)
}

fn parse_range_rule(node: &roxmltree::Node) -> Result<RangeRule, String> {
    let id = node
        .attribute("id")
        .ok_or("<sce:range> must have 'id' attribute")?
        .to_string();

    let min = node.attribute("min").map(|s| s.to_string());
    let max = node.attribute("max").map(|s| s.to_string());

    if min.is_none() && max.is_none() {
        return Err(format!(
            "<sce:range id='{id}'> must have at least 'min' or 'max' attribute"
        ));
    }

    Ok(RangeRule { id, min, max })
}

fn parse_rate_of_change_rule(node: &roxmltree::Node) -> Result<RateOfChangeRule, String> {
    let id = node
        .attribute("id")
        .ok_or("<sce:rate-of-change> must have 'id' attribute")?
        .to_string();

    let max_delta = node
        .attribute("max-delta")
        .ok_or("<sce:rate-of-change> must have 'max-delta' attribute")?
        .to_string();

    let sample_interval_str = sce_attr(node, "sample-interval")
        .unwrap_or_else(|| "100ms".to_string());

    let sample_interval_ms = parse_time_interval(&sample_interval_str)?;

    Ok(RateOfChangeRule {
        id,
        max_delta,
        sample_interval_ms,
    })
}

/// Parse time interval like "100ms" or "1s" into milliseconds.
fn parse_time_interval(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(ms_str) = s.strip_suffix("ms") {
        ms_str
            .parse::<u32>()
            .map_err(|_| format!("Invalid time interval: '{s}'"))
    } else if let Some(s_str) = s.strip_suffix('s') {
        s_str
            .parse::<u32>()
            .map(|secs| secs * 1000)
            .map_err(|_| format!("Invalid time interval: '{s}'"))
    } else {
        Err(format!(
            "Time interval must end with 'ms' or 's', got: '{s}'"
        ))
    }
}

// ── Shared helpers ─────────────────────────────────────────────

/// Read an `sce:xxx` attribute from a node (namespace-qualified).
fn sce_attr(node: &roxmltree::Node, local_name: &str) -> Option<String> {
    node.attribute((SCE_NAMESPACE, local_name))
        .map(|s| s.to_string())
}

/// Parse `<sce:entry key="..." value="..."/>` children from a node.
fn parse_sce_entries(node: &roxmltree::Node) -> Result<Vec<LookupEntry>, String> {
    let mut entries = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "entry" && child.tag_name().namespace() == Some(SCE_NAMESPACE) {
            let key = child
                .attribute("key")
                .ok_or("<sce:entry> must have a 'key' attribute")?
                .to_string();
            let value = child
                .attribute("value")
                .ok_or("<sce:entry> must have a 'value' attribute")?
                .to_string();
            entries.push(LookupEntry { key, value });
        }
    }
    Ok(entries)
}

/// Parse a typed forge field from a <data> element.
fn parse_forge_field(data: &roxmltree::Node) -> Result<ForgeField, String> {
    let id = data
        .attribute("id")
        .ok_or("Forge <data> field must have an 'id' attribute")?
        .to_string();

    let type_str = sce_attr(data, "type")
        .ok_or_else(|| format!("Field '{id}' must have an 'sce:type' attribute"))?;
    let sce_type = SceType::from_attr(&type_str)
        .ok_or_else(|| format!("Unknown sce:type '{type_str}' on field '{id}'"))?;

    let dir_str = sce_attr(data, "direction")
        .ok_or_else(|| format!("Field '{id}' must have an 'sce:direction' attribute"))?;
    let direction = Direction::from_attr(&dir_str)
        .ok_or_else(|| format!("Unknown sce:direction '{dir_str}' on field '{id}'"))?;

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
