// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge parser — extracts kind-specific models from Extended SCXML.
//
// Reads `sce:kind` on <scxml> root and dispatches to kind-specific parsing.
// Also handles inline kinds on <data> elements within statechart documents.

use crate::forge::error::{ForgeError, Located, ValidationError, XmlError};
use crate::forge::model::*;
use crate::DocumentLabel;

/// Construct a [`Located<ForgeError>`] from a node + the enclosing
/// document's diagnostic label.
///
/// The `name` parameter here plays the `diagnostic_label` role of
/// [`DocumentLabel`] — it ends up verbatim in `location.file` on the
/// wire. Callers must never pass a pure identifier (model name / stem),
/// or downstream tooling loses the `.scxml` suffix it needs to open
/// the source.
///
/// Uses `roxmltree::Document::text_pos_at` on the node's byte range
/// start to recover the (line, col) from libxml2's point of view.
/// Every raise-site in this module that has a node in scope runs
/// through here so location data on diagnostics is uniform: an agent
/// reading `xml/schema-validation` and `validation/missing-attribute`
/// records gets the same shape of location hint for both.
fn located<E: Into<ForgeError>>(
    node: &roxmltree::Node,
    name: &str,
    err: E,
) -> Located<ForgeError> {
    let pos = node.document().text_pos_at(node.range().start);
    Located::new(err.into(), name, Some(pos.row), Some(pos.col))
}

/// Build a `Located<ForgeError>` from a stored line number rather than
/// a live `roxmltree::Node`. Used by post-loop validators whose anchor
/// element is no longer in scope but whose line was captured during
/// parsing (e.g. `ProcedureTransition.line`, `ProcedureState.line`).
fn located_at_line<E: Into<ForgeError>>(
    name: &str,
    line: Option<u32>,
    err: E,
) -> Located<ForgeError> {
    Located::new(err.into(), name, line, None)
}

/// Detect the `sce:kind` attribute on the <scxml> root element.
/// Returns `None` if no `sce:kind` is present (defaults to statechart).
///
/// Errors carry a file name but not always a line — an XML parse
/// failure is raised before the DOM exists, so no node anchors the
/// location. Callers that need a file label should pass one via
/// the higher-level `parse_forge_with_imports` entry point; this
/// lower-level helper reports the file-less version.
pub fn detect_kind(content: &str) -> Result<Option<ForgeKind>, ForgeError> {
    let doc = roxmltree::Document::parse(content)
        .map_err(|e| XmlError::Parse(e.to_string()))?;
    let root = doc.root_element();
    Ok(detect_kind_from_node(&root)?)
}

/// Single-parse entry point: detect kind and parse forge document in one pass.
/// Returns `None` if the document is a statechart (no `sce:kind` or `sce:kind="statechart"`).
pub fn parse_forge(
    content: &str,
    label: DocumentLabel<'_>,
) -> Result<Option<ForgeDocument>, Located<ForgeError>> {
    parse_forge_with_imports(content, label).map(|opt| opt.map(|pf| pf.document))
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
pub fn parse_forge_with_imports(
    content: &str,
    label: DocumentLabel<'_>,
) -> Result<Option<ParsedForge>, Located<ForgeError>> {
    // Diagnostic label surfaces in `XsdErrors.source_label` and every
    // `Located<ForgeError>::file`. The identifier only kicks in when
    // a kind-specific parser writes it into a model `name` field
    // (e.g. `TransformModel.name`). Keeping the two roles separated
    // from the top of the pipeline means downstream callers cannot
    // accidentally fold a `.scxml` suffix into generated symbols or
    // drop one from wire diagnostics.
    let diag = label.diagnostic_label;
    crate::forge::xsd_validator::validate_or_skip(content, diag)
        .map_err(|e| Located::new(XmlError::SchemaValidation(e).into(), diag, None, None))?;

    let doc = roxmltree::Document::parse(content).map_err(|e| {
        Located::new(XmlError::Parse(e.to_string()).into(), diag, None, None)
    })?;
    let root = doc.root_element();

    let kind = match detect_kind_from_node(&root) {
        Ok(None) => return Ok(None),
        Ok(Some(ForgeKind::Statechart)) => return Ok(None),
        Ok(Some(k)) => k,
        Err(e) => return Err(located(&root, diag, e)),
    };

    if !kind.is_supported() {
        return Err(located(
            &root,
            diag,
            ValidationError::UnsupportedKind(kind.to_string()),
        ));
    }

    let imports = parse_imports(&root, diag)?;
    let document = parse_forge_from_node(&root, label, kind)?;
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
    label: DocumentLabel<'_>,
    kind: ForgeKind,
) -> Result<ForgeDocument, Located<ForgeError>> {
    match kind {
        ForgeKind::Transform => parse_transform(root, label).map(ForgeDocument::Transform),
        ForgeKind::Lookup => parse_lookup(root, label).map(ForgeDocument::Lookup),
        ForgeKind::Condition => parse_condition(root, label).map(ForgeDocument::Condition),
        ForgeKind::Codec => parse_codec(root, label).map(ForgeDocument::Codec),
        ForgeKind::Validator => parse_validator(root, label).map(ForgeDocument::Validator),
        ForgeKind::Procedure => parse_procedure(root, label).map(ForgeDocument::Procedure),
        ForgeKind::Filter => parse_filter(root, label).map(ForgeDocument::Filter),
        ForgeKind::Interpolation => parse_interpolation(root, label).map(ForgeDocument::Interpolation),
        ForgeKind::Timer => parse_timer(root, label).map(ForgeDocument::Timer),
        ForgeKind::Observer => parse_observer(root, label).map(ForgeDocument::Observer),
        ForgeKind::Algorithm => parse_algorithm(root, label).map(ForgeDocument::Algorithm),
        ForgeKind::Statechart => Err(located(
            root,
            label.diagnostic_label,
            ValidationError::WrongPipeline {
                kind: ForgeKind::Statechart,
            },
        )),
    }
}

// ── Kind-parser surface ────────────────────────────────────────
//
// Every `parse_X` below returns `Result<_, Located<ForgeError>>` and
// raises through the shared `located()` helper at each failure site,
// anchoring the diagnostic at the most specific DOM node the error
// refers to (a particular `<data>`, a `<state>`, the `<datamodel>`
// container, or the `<scxml>` root when nothing more precise is in
// scope). Upstream agents branch on `stage + location` — so keeping
// the line data tight to the offending element is the contract this
// layer exists to serve.
//
// **Label contract** — each top-level kind parser takes `label:
// DocumentLabel<'_>` and threads the two roles explicitly at every
// use site:
//
//     fn parse_X(root, label: DocumentLabel<'_>) -> ... {
//         let datamodel = find_child(root, "datamodel").ok_or_else(||
//             located(root, label.diagnostic_label, …))?;
//         // …
//         Ok(XModel { name: label.identifier.to_string(), … })
//     }
//
// No local alias. `label.diagnostic_label` goes into every `located()`
// and helper that writes `location.file`; `label.identifier` is used
// once at the model construction site. Mixing the two — threading
// `label.diagnostic_label` into model `name` — would fold `.scxml`
// into generated Go/C++/Kotlin symbols.




// ── Transform parsing ──────────────────────────────────────────

fn parse_transform(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<TransformModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel")
        .ok_or_else(|| located(root, label.diagnostic_label, ValidationError::MissingElement {
            kind: ForgeKind::Transform,
            element: "datamodel".into(),
        }))?;

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data, label.diagnostic_label)?;
        match field.direction {
            Direction::In => inputs.push(field),
            Direction::Out => outputs.push(field),
            Direction::Internal => {
                return Err(located(&data, label.diagnostic_label, ValidationError::InvalidDirection {
                    kind: ForgeKind::Transform,
                    direction: "internal".into(),
                    field: field.id,
                }));
            }
        }
    }

    if inputs.is_empty() {
        return Err(located(&datamodel, label.diagnostic_label, ValidationError::EmptyCollection {
            kind: ForgeKind::Transform,
            what: "input field".into(),
        }));
    }
    if outputs.is_empty() {
        return Err(located(&datamodel, label.diagnostic_label, ValidationError::EmptyCollection {
            kind: ForgeKind::Transform,
            what: "output field".into(),
        }));
    }

    for out in &outputs {
        if out.expr.is_none() {
            return Err(located(&datamodel, label.diagnostic_label, ValidationError::MissingAttribute {
                element: format!("Transform output field '{}'", out.id),
                attr: "expr".into(),
            }));
        }
    }

    Ok(TransformModel {
        name: label.identifier.to_string(),
        inputs,
        outputs,
    })
}

// ── Lookup parsing ─────────────────────────────────────────────

fn parse_lookup(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<LookupModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Lookup,
                element: "datamodel".into(),
            },
        )
    })?;

    // Carry the source `<data>` node alongside the value so the
    // post-loop policy validators can anchor diagnostics at the
    // element that declared `sce:default` / `sce:on-miss` instead of
    // collapsing to the surrounding `<datamodel>`. Node is `Copy`,
    // so the struct is cheap to pass around.
    struct DataAttr<'a, 'input: 'a> {
        value: String,
        node: roxmltree::Node<'a, 'input>,
    }

    let mut input: Option<ForgeField> = None;
    let mut output: Option<ForgeField> = None;
    let mut entries = Vec::new();
    let mut explicit_default: Option<DataAttr> = None;
    let mut on_miss_attr: Option<DataAttr> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            input = Some(parse_forge_field(&data, label.diagnostic_label)?);
        } else if dir.as_deref() == Some("out") {
            output = Some(parse_forge_field(&data, label.diagnostic_label)?);
        } else {
            if let Some(def) = sce_attr(&data, "default") {
                explicit_default = Some(DataAttr { value: def, node: data });
            }
            if let Some(oms) = sce_attr(&data, "on-miss") {
                on_miss_attr = Some(DataAttr { value: oms, node: data });
            }
            entries.extend(parse_sce_entries(&data, label.diagnostic_label)?);
        }
    }

    let input = input.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Lookup,
                element: "input field (sce:direction=\"in\")".into(),
            },
        )
    })?;
    let output = output.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Lookup,
                element: "output field (sce:direction=\"out\")".into(),
            },
        )
    })?;

    if entries.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Lookup,
                what: "<sce:entry>".into(),
            },
        ));
    }

    // Key uniqueness is enforced inside parse_sce_entries while the
    // offending `<sce:entry>` node is still in scope — the raise
    // anchors at the duplicate row rather than at the surrounding
    // `<datamodel>`.

    // Bind directly on the `Option<DataAttr>` so each arm has the
    // declaring `<data>` node in scope without an unwrap downstream.
    let miss_policy = match on_miss_attr {
        Some(oms) if oms.value == "error" => {
            if let Some(def) = explicit_default {
                // Anchor at the `<data>` that declared `sce:default` —
                // it's the element the agent must edit to resolve the
                // conflict (deleting the default, or relaxing the
                // policy on the on-miss element).
                return Err(located(
                    &def.node,
                    label.diagnostic_label,
                    ValidationError::IncompatibleAttributes {
                        element: "Lookup".into(),
                        detail: "sce:on-miss=\"error\" is incompatible with sce:default; \
                                 an error policy has no fallback value"
                            .into(),
                    },
                ));
            }
            MissPolicy::Error
        }
        Some(oms) if oms.value == "default" => {
            let value = explicit_default
                .map(|da| da.value)
                .unwrap_or_else(|| entries[0].value.clone());
            MissPolicy::Default(value)
        }
        None => {
            // Absent attribute matches "default": fall back to the
            // explicit sce:default value, or the first entry if none.
            let value = explicit_default
                .map(|da| da.value)
                .unwrap_or_else(|| entries[0].value.clone());
            MissPolicy::Default(value)
        }
        Some(oms) => {
            // Unknown value — anchor at the declaring `<data>`.
            return Err(located(
                &oms.node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "Lookup".into(),
                    attr: "sce:on-miss".into(),
                    value: oms.value,
                    expected: "default, error".into(),
                },
            ));
        }
    };

    Ok(LookupModel {
        name: label.identifier.to_string(),
        input,
        output,
        entries,
        miss_policy,
    })
}

// ── Condition parsing ──────────────────────────────────────────

fn parse_condition(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<ConditionModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Condition,
                element: "datamodel".into(),
            },
        )
    })?;

    let mut inputs = Vec::new();
    let mut expr = String::new();

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data, label.diagnostic_label)?;
        match field.direction {
            Direction::In => inputs.push(field),
            Direction::Out => {
                if let Some(e) = &field.expr {
                    expr = e.clone();
                } else {
                    return Err(located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: "Condition output field".into(),
                            attr: "expr".into(),
                        },
                    ));
                }
            }
            Direction::Internal => {
                return Err(located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::InvalidDirection {
                        kind: ForgeKind::Condition,
                        direction: "internal".into(),
                        field: field.id,
                    },
                ));
            }
        }
    }

    if inputs.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Condition,
                what: "input field".into(),
            },
        ));
    }
    if expr.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Condition,
                element: "output field with an 'expr' attribute".into(),
            },
        ));
    }

    Ok(ConditionModel {
        name: label.identifier.to_string(),
        inputs,
        expr,
    })
}

// ── Codec parsing ──────────────────────────────────────────────

fn parse_codec(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<CodecModel, Located<ForgeError>> {
    let default_endian = sce_attr(root, "default-endian")
        .and_then(|s| Endian::from_attr(&s))
        .unwrap_or(Endian::Big);

    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Codec,
                element: "datamodel".into(),
            },
        )
    })?;

    let mut fields = Vec::new();
    let mut input_length: Option<u32> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            if let Some(len_str) = sce_attr(&data, "length") {
                input_length = Some(parse_int(&len_str).ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::NumericParse {
                            element: "Codec input".into(),
                            attr: "sce:length".into(),
                            value: len_str,
                            detail: "expected integer".into(),
                        },
                    )
                })?);
            }
            continue;
        }

        // Output fields with byte layout (on <data> elements).
        if sce_attr(&data, "byte").is_some() {
            fields.push(parse_codec_field_from_node(&data, label.diagnostic_label)?);
        }
    }

    // Also check for <sce:field> elements (used in both standalone and inline codec)
    // and <sce:flags> containers (RFC §5.B B1-γ — same wire shape as a
    // plain unsigned-int field plus named-bit accessors emitted by codegen).
    for child in datamodel.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        match child.tag_name().name() {
            "field" => {
                fields.push(parse_codec_field_from_node(&child, label.diagnostic_label)?);
            }
            "flags" => {
                fields.push(parse_codec_flags_from_node(&child, label.diagnostic_label)?);
            }
            _ => {}
        }
    }

    if fields.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Codec,
                what: "field with byte layout".into(),
            },
        ));
    }

    // RFC §5.B variant primitive (B1-β): optional <sce:variant> suffix
    // under <datamodel>. Resolves the tag field reference against the
    // codec's own field list; arm body aliases (resolved against
    // <sce:import> aliases) are validated downstream by the codegen
    // step which has the import set.
    let variant = parse_codec_variant(&datamodel, &fields, label)?;

    Ok(CodecModel {
        name: label.identifier.to_string(),
        default_endian,
        input_length,
        fields,
        variant,
    })
}

/// RFC §5.B variant primitive — parse `<sce:variant>` element under
/// `<datamodel>`. Returns `None` when the codec has no variant suffix.
///
/// Validates intra-codec references (`tag=` resolves to a field, and
/// the field type is unsigned-int). Arm body alias resolution against
/// `<sce:import>` aliases happens downstream at codegen time when the
/// import set is in scope.
///
/// Fires `codec/variant-arm-unreachable` when no `<sce:default>` arm
/// is declared and the enumerated arms don't cover the tag field's
/// value domain. v1 considers uint8 (256 values) and uint16 (65536
/// values) practically enumerable; uint32 / uint64 always require a
/// default arm.
fn parse_codec_variant(
    datamodel: &roxmltree::Node,
    fields: &[CodecField],
    label: DocumentLabel<'_>,
) -> Result<Option<CodecVariant>, Located<ForgeError>> {
    let variant_node = match datamodel.children().find(|n| {
        n.is_element()
            && n.tag_name().name() == "variant"
            && n.tag_name().namespace() == Some(SCE_NAMESPACE)
    }) {
        Some(n) => n,
        None => return Ok(None),
    };

    // RFC §5.B uses unqualified attributes on <sce:variant>/<sce:arm>/
    // <sce:default> child elements (matches the <sce:entry key="..."/>
    // convention for SCE-element-internal attributes; SCE-namespaced
    // attributes are reserved for attributes declared on non-SCE host
    // elements like <data sce:byte=...>).
    let tag_field = variant_node
        .attribute("tag")
        .ok_or_else(|| {
            located(
                &variant_node,
                label.diagnostic_label,
                ValidationError::MissingAttribute {
                    element: "<sce:variant>".into(),
                    attr: "tag".into(),
                },
            )
        })?
        .to_string();

    // Resolve tag against the codec's own fields and capture its type
    // for arm-domain reasoning. The tag field MUST be unsigned-int
    // (uint8/uint16/uint32/uint64) because the arm `value=` matches a
    // wire-decoded unsigned scalar; signed / bytes / float tags have
    // no valid discriminator semantics.
    let tag_type = match fields.iter().find(|f| f.id == tag_field) {
        Some(f) if f.sce_type.is_unsigned() => f.sce_type.clone(),
        Some(f) => {
            return Err(located(
                &variant_node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "<sce:variant>".into(),
                    attr: "sce:tag".into(),
                    value: tag_field.clone(),
                    expected: format!(
                        "tag field must be unsigned-int (uint8/uint16/uint32/uint64); '{tag_field}' is {:?}",
                        f.sce_type
                    ),
                },
            ));
        }
        None => {
            let available: Vec<String> = fields.iter().map(|f| f.id.clone()).collect();
            return Err(located(
                &variant_node,
                label.diagnostic_label,
                ValidationError::InvalidReference {
                    kind: ForgeKind::Codec,
                    name: tag_field.clone(),
                    what: "field".into(),
                    available: available.join(", "),
                },
            ));
        }
    };

    let mut arms: Vec<VariantArm> = Vec::new();
    let mut default_arm: Option<VariantArm> = None;

    for child in variant_node.children().filter(|n| n.is_element()) {
        let local = child.tag_name().name();
        let ns = child.tag_name().namespace();
        if ns != Some(SCE_NAMESPACE) {
            continue;
        }
        match local {
            "arm" => {
                let value_str = child.attribute("value").ok_or_else(|| {
                    located(
                        &child,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: "<sce:arm>".into(),
                            attr: "value".into(),
                        },
                    )
                })?;
                let value = parse_int_u64(value_str).ok_or_else(|| {
                    located(
                        &child,
                        label.diagnostic_label,
                        ValidationError::NumericParse {
                            element: "<sce:arm>".into(),
                            attr: "value".into(),
                            value: value_str.to_string(),
                            detail: "expected unsigned integer (decimal or 0x-hex)".into(),
                        },
                    )
                })?;
                let body_alias = child
                    .attribute("type")
                    .ok_or_else(|| {
                        located(
                            &child,
                            label.diagnostic_label,
                            ValidationError::MissingAttribute {
                                element: format!("<sce:arm value=\"{value_str}\">"),
                                attr: "type".into(),
                            },
                        )
                    })?
                    .to_string();
                arms.push(VariantArm { value, body_alias });
            }
            "default" => {
                if default_arm.is_some() {
                    return Err(located(
                        &child,
                        label.diagnostic_label,
                        ValidationError::SingletonViolation {
                            kind: ForgeKind::Codec,
                            attr: "<sce:default>".into(),
                        },
                    ));
                }
                let body_alias = child
                    .attribute("type")
                    .ok_or_else(|| {
                        located(
                            &child,
                            label.diagnostic_label,
                            ValidationError::MissingAttribute {
                                element: "<sce:default>".into(),
                                attr: "type".into(),
                            },
                        )
                    })?
                    .to_string();
                // The default arm carries no compile-time discriminator;
                // its runtime tag value is preserved on the decoded
                // sum-type variant by the codegen. v1 stores `value: 0`
                // as a sentinel — codegen never reads this field for
                // default arms (it dispatches via the catch-all branch).
                default_arm = Some(VariantArm { value: 0, body_alias });
            }
            _ => {
                return Err(located(
                    &child,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "<sce:variant>".into(),
                        attr: "child element".into(),
                        value: local.to_string(),
                        expected: "<sce:arm> or <sce:default>".into(),
                    },
                ));
            }
        }
    }

    if arms.is_empty() && default_arm.is_none() {
        return Err(located(
            &variant_node,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Codec,
                what: "<sce:arm> child of <sce:variant>".into(),
            },
        ));
    }

    // RFC §5.B `codec/variant-arm-unreachable`: when no <sce:default>
    // arm is declared, the enumerated arms must cover the tag field's
    // entire value domain — otherwise some incoming tag value would
    // reach the runtime decoder with no matching branch. v1 considers
    // uint8 (256) and uint16 (65536) practically enumerable; uint32 /
    // uint64 always require a default.
    if default_arm.is_none() {
        let domain_size: Option<u64> = match tag_type {
            SceType::Uint8 => Some(256),
            SceType::Uint16 => Some(65_536),
            _ => None,
        };
        let arm_count = arms.len();
        let exhaustive = match domain_size {
            Some(n) => (arm_count as u64) >= n,
            None => false,
        };
        if !exhaustive {
            return Err(located(
                &variant_node,
                label.diagnostic_label,
                ValidationError::CodecVariantArmUnreachable {
                    codec: label.identifier.to_string(),
                    tag_field: tag_field.clone(),
                    tag_type: format!("{tag_type:?}").to_lowercase(),
                    arm_count,
                    domain_size,
                },
            ));
        }
    }

    Ok(Some(CodecVariant {
        tag_field,
        arms,
        default_arm,
    }))
}

/// Unified codec field parser — works for both `<data>` and `<sce:field>` elements.
///
/// Public so the statechart parser can reuse it for inline codec
/// extraction; takes `doc_name` so every raise anchors at the field's
/// own node line. Callers that don't have a meaningful document name
/// (e.g. inline-codec extraction inside the statechart parser) may
/// pass a label like `"<inline codec>"`.
pub fn parse_codec_field_from_node(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<CodecField, Located<ForgeError>> {
    let id = node
        .attribute("id")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "Codec field".into(),
                    attr: "id".into(),
                },
            )
        })?
        .to_string();

    let sce_type_str = sce_attr(node, "type").unwrap_or_else(|| "uint8".to_string());
    let sce_type = SceType::from_attr(&sce_type_str).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("field '{id}'"),
                attr: "sce:type".into(),
                value: sce_type_str.clone(),
                expected: "uint8, uint16, uint32, int8, int16, int32, float32, float64, bool, string, bytes".into(),
            },
        )
    })?;

    let byte_offset_str = sce_attr(node, "byte").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("Codec field '{id}'"),
                attr: "sce:byte".into(),
            },
        )
    })?;
    let byte_offset = parse_int(&byte_offset_str).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::NumericParse {
                element: format!("field '{id}'"),
                attr: "sce:byte".into(),
                value: byte_offset_str.clone(),
                detail: "expected integer".into(),
            },
        )
    })?;

    let bit_offset = sce_attr(node, "bit-offset").and_then(|s| parse_int(&s));

    let bit_size = {
        let bs = sce_attr(node, "bit-size").ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: format!("Codec field '{id}'"),
                    attr: "sce:bit-size".into(),
                },
            )
        })?;
        match bs.as_str() {
            "tail" => BitSize::Tail,
            "length-ref" => BitSize::LengthRef,
            "vle" => {
                // VLE bit-size pairs with the value type to derive the
                // continuation-chain cap (RFC §5.B Appendix B). Only
                // unsigned ints are valid carriers; B1-α ships u16/u32/u64.
                let width_bits = match sce_type {
                    SceType::Uint16 => 16,
                    SceType::Uint32 => 32,
                    SceType::Uint64 => 64,
                    _ => {
                        return Err(located(
                            node,
                            doc_name,
                            ValidationError::InvalidAttribute {
                                element: format!("field '{id}'"),
                                attr: "sce:bit-size".into(),
                                value: "vle".into(),
                                expected: "vle requires sce:type ∈ {uint16, uint32, uint64}".into(),
                            },
                        ));
                    }
                };
                BitSize::Vle { width_bits }
            }
            _ => {
                let n = parse_int(&bs).ok_or_else(|| {
                    located(
                        node,
                        doc_name,
                        ValidationError::NumericParse {
                            element: format!("field '{id}'"),
                            attr: "sce:bit-size".into(),
                            value: bs.clone(),
                            detail: "expected integer, 'tail', 'length-ref', or 'vle'".into(),
                        },
                    )
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
        flags: Vec::new(),
    })
}

/// RFC §5.B B1-γ flags primitive — parse `<sce:flags id=... sce:type=...
/// sce:byte=... sce:bit-size=N>` with `<sce:flag name="X" bit="N"/>`
/// child decls. Reuses [`parse_codec_field_from_node`] for the carrier
/// field (same byte-layout attrs as a plain `<sce:field>`), then walks
/// `<sce:flag>` children. Validates: carrier type is unsigned-int (so
/// shift / mask have well-defined wire semantics), each `bit=` lies
/// within the carrier's bit width, and flag names are unique within
/// the container.
fn parse_codec_flags_from_node(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<CodecField, Located<ForgeError>> {
    let mut field = parse_codec_field_from_node(node, doc_name)?;

    // Reject signed / float / bool / string / bytes carriers — bit
    // accessors would have undefined semantics under shift / mask.
    if !field.sce_type.is_unsigned() {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("<sce:flags id='{}'>", field.id),
                attr: "sce:type".into(),
                value: format!("{:?}", field.sce_type).to_lowercase(),
                expected: "uint8 / uint16 / uint32 / uint64".into(),
            },
        ));
    }

    let bit_width = field.sce_type.int_bit_width().expect("unsigned ⇒ Some");

    let mut seen_names: std::collections::BTreeSet<String> = Default::default();
    let mut flag_defs: Vec<FlagDef> = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE)
            || child.tag_name().name() != "flag"
        {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flags id='{}'>", field.id),
                    attr: "child element".into(),
                    value: child.tag_name().name().to_string(),
                    expected: "<sce:flag>".into(),
                },
            ));
        }
        let name = child
            .attribute("name")
            .ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: "<sce:flag>".into(),
                        attr: "name".into(),
                    },
                )
            })?
            .to_string();
        if !seen_names.insert(name.clone()) {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag> in <sce:flags id='{}'>", field.id),
                    attr: "name".into(),
                    value: name.clone(),
                    expected: "unique within parent <sce:flags>".into(),
                },
            ));
        }
        let bit_str = child.attribute("bit").ok_or_else(|| {
            located(
                &child,
                doc_name,
                ValidationError::MissingAttribute {
                    element: format!("<sce:flag name='{name}'>"),
                    attr: "bit".into(),
                },
            )
        })?;
        let bit = parse_int(bit_str).ok_or_else(|| {
            located(
                &child,
                doc_name,
                ValidationError::NumericParse {
                    element: format!("<sce:flag name='{name}'>"),
                    attr: "bit".into(),
                    value: bit_str.to_string(),
                    detail: "expected non-negative integer".into(),
                },
            )
        })?;
        if bit >= bit_width {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag name='{name}'>"),
                    attr: "bit".into(),
                    value: bit.to_string(),
                    expected: format!("0..{bit_width} (carrier is {bit_width}-bit)"),
                },
            ));
        }
        flag_defs.push(FlagDef { name, bit });
    }

    if flag_defs.is_empty() {
        return Err(located(
            node,
            doc_name,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Codec,
                what: format!("<sce:flag> child of <sce:flags id='{}'>", field.id),
            },
        ));
    }

    field.flags = flag_defs;
    Ok(field)
}

// ── Validator parsing ──────────────────────────────────────

fn parse_validator(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<ValidatorModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Validator,
                element: "datamodel".into(),
            },
        )
    })?;

    let mut inputs = Vec::new();
    let mut ranges = Vec::new();
    let mut rate_of_changes = Vec::new();
    let mut plausibility: Option<String> = None;

    for data in data_children(&datamodel) {
        let field = parse_forge_field(&data, label.diagnostic_label)?;
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
                    let sample_interval_ms = parse_time_interval(&sample_interval_str)
                        .map_err(|e| located(&data, label.diagnostic_label, e))?;
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
                        return Err(located(
                            &data,
                            label.diagnostic_label,
                            ValidationError::SingletonViolation {
                                kind: ForgeKind::Validator,
                                attr: "sce:plausibility".into(),
                            },
                        ));
                    }
                    plausibility = Some(expr);
                }
            }
            Direction::Internal => {
                return Err(located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::InvalidDirection {
                        kind: ForgeKind::Validator,
                        direction: "internal".into(),
                        field: field.id,
                    },
                ));
            }
        }
    }

    if inputs.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Validator,
                what: "input field".into(),
            },
        ));
    }

    if ranges.is_empty() && rate_of_changes.is_empty() && plausibility.is_none() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Validator,
                what: "rule (sce:range-min/max, sce:max-delta, or sce:plausibility)".into(),
            },
        ));
    }

    Ok(ValidatorModel {
        name: label.identifier.to_string(),
        inputs,
        rules: ValidatorRules {
            ranges,
            rate_of_changes,
            plausibility,
        },
    })
}

// ── Procedure parsing ──────────────────────────────────────

fn parse_procedure(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<ProcedureModel, Located<ForgeError>> {
    let initial = root
        .attribute("initial")
        .ok_or_else(|| {
            located(
                root,
                label.diagnostic_label,
                ValidationError::MissingAttribute {
                    element: "Procedure <scxml>".into(),
                    attr: "initial".into(),
                },
            )
        })?
        .to_string();

    // Parse input and internal fields from <datamodel>
    let mut inputs = Vec::new();
    let mut internals = Vec::new();
    let mut helpers = Vec::new();
    if let Some(datamodel) = find_child(root, "datamodel") {
        for data in data_children(&datamodel) {
            let field = parse_forge_field(&data, label.diagnostic_label)?;
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
            let helper = parse_procedure_helper(&child, label.diagnostic_label)?;
            if helpers.iter().any(|h: &ProcedureHelper| h.name == helper.name) {
                return Err(located(
                    &child,
                    label.diagnostic_label,
                    ValidationError::DuplicateId {
                        kind: ForgeKind::Procedure,
                        what: "<sce:helper>".into(),
                        id: helper.name,
                    },
                ));
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
            .ok_or_else(|| {
                located(
                    &child,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: format!("<{tag}>"),
                        attr: "id".into(),
                    },
                )
            })?
            .to_string();

        if !state_ids.insert(id.clone()) {
            return Err(located(
                &child,
                label.diagnostic_label,
                ValidationError::DuplicateId {
                    kind: ForgeKind::Procedure,
                    what: "state id".into(),
                    id,
                },
            ));
        }

        let transitions = if is_final {
            Vec::new()
        } else {
            parse_procedure_transitions(&child, label.diagnostic_label)?
        };

        // Parse <onentry> → <send> actions
        let on_entry_sends = parse_procedure_onentry(&child, label.diagnostic_label)?;

        // Parse <donedata> on <final> elements
        let done_params = if is_final {
            parse_procedure_donedata(&child, label.diagnostic_label)?
        } else {
            Vec::new()
        };

        let line = Some(child.document().text_pos_at(child.range().start).row);
        states.push(ProcedureState {
            id,
            is_final,
            transitions,
            on_entry_sends,
            done_params,
            line,
        });
    }

    if states.is_empty() {
        return Err(located(
            root,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Procedure,
                what: "<state> or <final> element".into(),
            },
        ));
    }

    // Validate: initial state must exist
    if !state_ids.contains(&initial) {
        return Err(located(
            root,
            label.diagnostic_label,
            ValidationError::InvalidReference {
                kind: ForgeKind::Procedure,
                name: initial.clone(),
                what: "state".into(),
                available: state_ids.iter().cloned().collect::<Vec<_>>().join(", "),
            },
        ));
    }

    // Validate: must have at least one final state
    if !states.iter().any(|s| s.is_final) {
        return Err(located(
            root,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Procedure,
                what: "<final> element".into(),
            },
        ));
    }

    // Validate: all transition targets must reference existing states.
    // Anchor the diagnostic at the offending <transition>'s own line —
    // ProcedureTransition carries it directly so we don't need a twin
    // `state_nodes` Vec whose lockstep invariant a future loop edit
    // could silently break.
    for state in &states {
        for tr in &state.transitions {
            if !state_ids.contains(&tr.target) {
                return Err(located_at_line(
                    label.diagnostic_label,
                    tr.line,
                    ValidationError::InvalidReference {
                        kind: ForgeKind::Procedure,
                        name: format!("transition target '{}' in state '{}'", tr.target, state.id),
                        what: "state".into(),
                        available: state_ids.iter().cloned().collect::<Vec<_>>().join(", "),
                    },
                ));
            }
        }
    }

    // Validate: non-final states must have at least one transition. The
    // raise anchors at the owning <state> via the line ProcedureState
    // captured during construction.
    for state in &states {
        if !state.is_final && state.transitions.is_empty() {
            return Err(located_at_line(
                label.diagnostic_label,
                state.line,
                ValidationError::EmptyCollection {
                    kind: ForgeKind::Procedure,
                    what: format!("<transition> in non-final state '{}'", state.id),
                },
            ));
        }
    }

    let model = ProcedureModel {
        name: label.identifier.to_string(),
        inputs,
        internals,
        helpers,
        initial,
        states,
    };

    // RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B1: catch
    // self-contradicting bytes max-size declarations before any
    // backend codegen runs. The runtime β path (error.execution
    // raised at the actual cap-violation site in the generated
    // procedure runtime) covers data exceeding a consistently
    // declared cap; this static pass covers the orthogonal class of
    // declarations whose source cap is *itself* larger than the
    // destination cap. Anchored at the procedure root so the
    // diagnostic carries its identifier in the message.
    if let Err(err) = crate::forge::validate::validate_bytes_max_size_consistency(&model) {
        return Err(located_at_line(label.diagnostic_label, None, err));
    }

    Ok(model)
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
fn parse_procedure_helper(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<ProcedureHelper, Located<ForgeError>> {
    let helper_name = node
        .attribute("name")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "<sce:helper>".into(),
                    attr: "name".into(),
                },
            )
        })?
        .to_string();
    if helper_name.is_empty() {
        return Err(located(
            node,
            doc_name,
            ValidationError::EmptyValue {
                element: "<sce:helper>".into(),
                attr: "name".into(),
            },
        ));
    }
    if !is_ident(&helper_name) {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:helper>".into(),
                attr: "name".into(),
                value: helper_name,
                expected: "[A-Za-z_][A-Za-z0-9_]* (valid identifier for 5-language generated source)".into(),
            },
        ));
    }
    let args_raw = node.attribute("args").unwrap_or("");
    let mut args = Vec::new();
    for part in args_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let sce_ty = SceType::from_attr(part).ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:helper name=\"{helper_name}\">"),
                    attr: "args".into(),
                    value: part.to_string(),
                    expected: "valid sce:type".into(),
                },
            )
        })?;
        args.push(sce_ty);
    }
    let returns_raw = node.attribute("returns").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("<sce:helper name=\"{helper_name}\">"),
                attr: "returns".into(),
            },
        )
    })?;
    let returns = SceType::from_attr(returns_raw).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("<sce:helper name=\"{helper_name}\">"),
                attr: "returns".into(),
                value: returns_raw.to_string(),
                expected: "valid sce:type".into(),
            },
        )
    })?;
    // RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B1: optional cap
    // on a bytes-typed return. Validator pass flags it if `returns` is
    // not bytes.
    let returns_max_size = sce_attr(node, "returns-max-size").and_then(|s| parse_int(&s));

    Ok(ProcedureHelper {
        name: helper_name,
        args,
        returns,
        returns_max_size,
    })
}

/// Parse <transition> children of a procedure state.
/// Level 1: target + cond only.
/// Level 2: + event + <assign> children.
fn parse_procedure_transitions(
    state: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<ProcedureTransition>, Located<ForgeError>> {
    let mut transitions = Vec::new();

    for child in state.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "transition" {
            continue;
        }

        let target = child
            .attribute("target")
            .ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: format!(
                            "<transition> in state '{}'",
                            state.attribute("id").unwrap_or("?")
                        ),
                        attr: "target".into(),
                    },
                )
            })?
            .to_string();

        let cond = child.attribute("cond").map(|s| s.to_string());
        let event = child.attribute("event").map(|s| s.to_string());

        // Parse <assign> children within the transition (Level 2)
        let assigns = parse_procedure_assigns(&child, doc_name)?;

        let line = Some(child.document().text_pos_at(child.range().start).row);
        transitions.push(ProcedureTransition {
            target,
            cond,
            event,
            assigns,
            line,
        });
    }

    Ok(transitions)
}

/// Parse <assign> children within a <transition> element.
fn parse_procedure_assigns(
    transition: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<ProcedureAssign>, Located<ForgeError>> {
    let mut assigns = Vec::new();
    for child in transition.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "assign" {
            continue;
        }
        let location_attr = child
            .attribute("location")
            .ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: "<assign>".into(),
                        attr: "location".into(),
                    },
                )
            })?
            .to_string();
        let expr = child
            .attribute("expr")
            .ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: "<assign>".into(),
                        attr: "expr".into(),
                    },
                )
            })?
            .to_string();
        assigns.push(ProcedureAssign {
            location: location_attr,
            expr,
        });
    }
    Ok(assigns)
}

/// Parse <onentry> → <send> actions within a procedure state.
fn parse_procedure_onentry(
    state: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<ProcedureSendAction>, Located<ForgeError>> {
    let mut sends = Vec::new();
    for onentry in state.children().filter(|n| n.is_element() && n.tag_name().name() == "onentry") {
        for child in onentry.children().filter(|n| n.is_element()) {
            if child.tag_name().name() != "send" {
                continue;
            }
            let service = sce_attr(&child, "service").ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: "<send> in procedure <onentry>".into(),
                        attr: "sce:service".into(),
                    },
                )
            })?;
            let subfunc = sce_attr(&child, "subfunc");
            let addr = sce_attr(&child, "addr");
            let payload = sce_attr(&child, "payload");
            // RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B1: cap on
            // the bytes the service handler may return as `_event.data`.
            let response_max_size = sce_attr(&child, "response-max-size")
                .and_then(|s| parse_int(&s));
            sends.push(ProcedureSendAction {
                service,
                subfunc,
                addr,
                payload,
                response_max_size,
            });
        }
    }
    Ok(sends)
}

/// Parse <donedata> → <param> children within a <final> element.
fn parse_procedure_donedata(
    final_elem: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<ProcedureDoneParam>, Located<ForgeError>> {
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
                .ok_or_else(|| {
                    located(
                        &child,
                        doc_name,
                        ValidationError::MissingAttribute {
                            element: "<param> in <donedata>".into(),
                            attr: "name".into(),
                        },
                    )
                })?
                .to_string();
            let expr = child
                .attribute("expr")
                .ok_or_else(|| {
                    located(
                        &child,
                        doc_name,
                        ValidationError::MissingAttribute {
                            element: "<param> in <donedata>".into(),
                            attr: "expr".into(),
                        },
                    )
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

fn parse_filter(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<FilterModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Filter,
                element: "datamodel".into(),
            },
        )
    })?;

    let mut input: Option<ForgeField> = None;
    let mut output: Option<ForgeField> = None;
    let mut output_node: Option<roxmltree::Node> = None;
    let mut filter_type: Option<FilterType> = None;
    let mut window: Option<u32> = None;
    let mut alpha: Option<f64> = None;

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");
        match dir.as_deref() {
            Some("in") => {
                input = Some(parse_forge_field(&data, label.diagnostic_label)?);
            }
            Some("out") => {
                output = Some(parse_forge_field(&data, label.diagnostic_label)?);
                output_node = Some(data);

                let ft_str = sce_attr(&data, "filter").ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: "Filter output".into(),
                            attr: "sce:filter".into(),
                        },
                    )
                })?;
                filter_type = Some(FilterType::from_attr(&ft_str).ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::InvalidAttribute {
                            element: "Filter output".into(),
                            attr: "sce:filter".into(),
                            value: ft_str.clone(),
                            expected: "moving-average, low-pass, debounce".into(),
                        },
                    )
                })?);

                window = sce_attr(&data, "window").and_then(|s| s.parse::<u32>().ok());
                alpha = sce_attr(&data, "alpha").and_then(|s| s.parse::<f64>().ok());
            }
            _ => {
                return Err(located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::InvalidDirection {
                        kind: ForgeKind::Filter,
                        direction: dir.unwrap_or_default(),
                        field: String::new(),
                    },
                ));
            }
        }
    }

    let input = input.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Filter,
                element: "input field (sce:direction=\"in\")".into(),
            },
        )
    })?;
    let output = output.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Filter,
                element: "output field (sce:direction=\"out\")".into(),
            },
        )
    })?;
    let filter_type = filter_type.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingAttribute {
                element: "Filter output".into(),
                attr: "sce:filter".into(),
            },
        )
    })?;

    // Validate required parameters per filter type. The `sce:filter`
    // attribute that determined which param is required lives on the
    // output `<data>`; anchor the diagnostic there so an agent can
    // jump to the offending element instead of the surrounding
    // `<datamodel>`. The `unwrap_or` is defensive — `filter_type`
    // requires a present output, so output_node is always populated
    // by the time we reach this match.
    let param_anchor = output_node.as_ref().unwrap_or(&datamodel);
    match filter_type {
        FilterType::MovingAverage => {
            if window.is_none() {
                return Err(located(
                    param_anchor,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: "Moving-average filter".into(),
                        attr: "sce:window".into(),
                    },
                ));
            }
        }
        FilterType::LowPass => {
            if alpha.is_none() {
                return Err(located(
                    param_anchor,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: "Low-pass filter".into(),
                        attr: "sce:alpha".into(),
                    },
                ));
            }
        }
        FilterType::Debounce => {
            if window.is_none() {
                return Err(located(
                    param_anchor,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: "Debounce filter".into(),
                        attr: "sce:window".into(),
                    },
                ));
            }
        }
    }

    Ok(FilterModel {
        name: label.identifier.to_string(),
        input,
        output,
        filter_type,
        window,
        alpha,
    })
}

// ── Interpolation parsing ─────────────────────────────────────

fn parse_interpolation(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<InterpolationModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Interpolation,
                element: "datamodel".into(),
            },
        )
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
                inputs.push(parse_forge_field(&data, label.diagnostic_label)?);
            }
            Some("out") => {
                output = Some(parse_forge_field(&data, label.diagnostic_label)?);

                let method_str = sce_attr(&data, "interpolation").ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: "Interpolation output".into(),
                            attr: "sce:interpolation".into(),
                        },
                    )
                })?;
                method = Some(InterpolationMethod::from_attr(&method_str).ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::InvalidAttribute {
                            element: "Interpolation output".into(),
                            attr: "sce:interpolation".into(),
                            value: method_str.clone(),
                            expected: "linear, bilinear".into(),
                        },
                    )
                })?);

                if let Some(oob_str) = sce_attr(&data, "out-of-bounds") {
                    out_of_bounds = OutOfBounds::from_attr(&oob_str).ok_or_else(|| {
                        located(
                            &data,
                            label.diagnostic_label,
                            ValidationError::InvalidAttribute {
                                element: "Interpolation output".into(),
                                attr: "sce:out-of-bounds".into(),
                                value: oob_str.clone(),
                                expected: "clamp, extrapolate, error".into(),
                            },
                        )
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
                        let breakpoints = breakpoints.map_err(|e| {
                            located(
                                &data,
                                label.diagnostic_label,
                                ValidationError::NumericParse {
                                    element: format!("Interpolation axis-{}", inp.id),
                                    attr: format!("sce:axis-{}", inp.id),
                                    value: bp_str.clone(),
                                    detail: e.to_string(),
                                },
                            )
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
                        .map_err(|e| {
                            located(
                                &data,
                                label.diagnostic_label,
                                ValidationError::NumericParse {
                                    element: "Interpolation output".into(),
                                    attr: "table values".into(),
                                    value: text.clone(),
                                    detail: e.to_string(),
                                },
                            )
                        })?;
                }
            }
            _ => {
                return Err(located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::InvalidDirection {
                        kind: ForgeKind::Interpolation,
                        direction: dir.unwrap_or_default(),
                        field: String::new(),
                    },
                ));
            }
        }
    }

    let output = output.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Interpolation,
                element: "output field (sce:direction=\"out\")".into(),
            },
        )
    })?;
    let method = method.ok_or_else(|| {
        located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::MissingAttribute {
                element: "Interpolation output".into(),
                attr: "sce:interpolation".into(),
            },
        )
    })?;

    if inputs.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Interpolation,
                what: "input field".into(),
            },
        ));
    }
    if axes.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Interpolation,
                what: "sce:axis-* attribute".into(),
            },
        ));
    }
    if values.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Interpolation,
                what: "table values in the output element text".into(),
            },
        ));
    }

    // Validate axis count matches method
    match method {
        InterpolationMethod::Linear => {
            if axes.len() != 1 {
                return Err(located(
                    &datamodel,
                    label.diagnostic_label,
                    ValidationError::CountMismatch {
                        kind: ForgeKind::Interpolation,
                        detail: "linear: requires exactly 1 axis".into(),
                    },
                ));
            }
            if values.len() != axes[0].breakpoints.len() {
                return Err(located(
                    &datamodel,
                    label.diagnostic_label,
                    ValidationError::CountMismatch {
                        kind: ForgeKind::Interpolation,
                        detail: format!(
                            "linear: value count ({}) must match axis breakpoints ({})",
                            values.len(),
                            axes[0].breakpoints.len()
                        ),
                    },
                ));
            }
        }
        InterpolationMethod::Bilinear => {
            if axes.len() != 2 {
                return Err(located(
                    &datamodel,
                    label.diagnostic_label,
                    ValidationError::CountMismatch {
                        kind: ForgeKind::Interpolation,
                        detail: "bilinear: requires exactly 2 axes".into(),
                    },
                ));
            }
            let expected = axes[0].breakpoints.len() * axes[1].breakpoints.len();
            if values.len() != expected {
                return Err(located(
                    &datamodel,
                    label.diagnostic_label,
                    ValidationError::CountMismatch {
                        kind: ForgeKind::Interpolation,
                        detail: format!(
                            "bilinear: value count ({}) must equal rows({}) x cols({}) = {}",
                            values.len(),
                            axes[0].breakpoints.len(),
                            axes[1].breakpoints.len(),
                            expected
                        ),
                    },
                ));
            }
        }
    }

    Ok(InterpolationModel {
        name: label.identifier.to_string(),
        inputs,
        output,
        method,
        out_of_bounds,
        axes,
        values,
    })
}

// ── Timer parsing ─────────────────────────────────────────────

fn parse_timer(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<TimerModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Timer,
                element: "datamodel".into(),
            },
        )
    })?;

    let mut timers = Vec::new();

    for data in data_children(&datamodel) {
        let timer_str = match sce_attr(&data, "timer") {
            Some(s) => s,
            None => continue,
        };

        let id = data
            .attribute("id")
            .ok_or_else(|| {
                located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: "Timer <data>".into(),
                        attr: "id".into(),
                    },
                )
            })?
            .to_string();

        let timer_type = TimerType::from_attr(&timer_str).ok_or_else(|| {
            located(
                &data,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: format!("Timer '{id}'"),
                    attr: "sce:timer".into(),
                    value: timer_str.clone(),
                    expected: "periodic, timeout, delayed".into(),
                },
            )
        })?;

        let time_ms = match timer_type {
            TimerType::Periodic => {
                let s = sce_attr(&data, "interval").ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: format!("Periodic timer '{id}'"),
                            attr: "sce:interval".into(),
                        },
                    )
                })?;
                s.parse::<u32>().map_err(|_| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::NumericParse {
                            element: format!("timer '{id}'"),
                            attr: "sce:interval".into(),
                            value: s.clone(),
                            detail: "expected integer".into(),
                        },
                    )
                })?
            }
            TimerType::Timeout => {
                let s = sce_attr(&data, "duration").ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: format!("Timeout timer '{id}'"),
                            attr: "sce:duration".into(),
                        },
                    )
                })?;
                s.parse::<u32>().map_err(|_| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::NumericParse {
                            element: format!("timer '{id}'"),
                            attr: "sce:duration".into(),
                            value: s.clone(),
                            detail: "expected integer".into(),
                        },
                    )
                })?
            }
            TimerType::Delayed => {
                let s = sce_attr(&data, "delay").ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: format!("Delayed timer '{id}'"),
                            attr: "sce:delay".into(),
                        },
                    )
                })?;
                s.parse::<u32>().map_err(|_| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::NumericParse {
                            element: format!("timer '{id}'"),
                            attr: "sce:delay".into(),
                            value: s.clone(),
                            detail: "expected integer".into(),
                        },
                    )
                })?
            }
        };

        let event = sce_attr(&data, "event");
        let on_timeout = sce_attr(&data, "on-timeout");

        if event.is_none() && on_timeout.is_none() {
            return Err(located(
                &data,
                label.diagnostic_label,
                ValidationError::RequireEither {
                    element: format!("Timer '{id}'"),
                    alternatives: vec!["sce:event".into(), "sce:on-timeout".into()],
                },
            ));
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
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Timer,
                what: "<data> with 'sce:timer' attribute".into(),
            },
        ));
    }

    Ok(TimerModel {
        name: label.identifier.to_string(),
        timers,
    })
}

// ── Observer parsing ──────────────────────────────────────────

fn parse_observer(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<ObserverModel, Located<ForgeError>> {
    let datamodel = find_child(root, "datamodel").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Observer,
                element: "datamodel".into(),
            },
        )
    })?;

    let event_domain = sce_attr(root, "event-domain");

    let mut inputs = Vec::new();
    let mut monitors = Vec::new();

    for data in data_children(&datamodel) {
        let dir = sce_attr(&data, "direction");

        if dir.as_deref() == Some("in") {
            inputs.push(parse_forge_field(&data, label.diagnostic_label)?);
            continue;
        }

        // Monitor definitions have sce:monitor attribute
        if let Some(monitor_type) = sce_attr(&data, "monitor") {
            if monitor_type != "threshold" {
                return Err(located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "Observer monitor".into(),
                        attr: "sce:monitor".into(),
                        value: monitor_type,
                        expected: "threshold".into(),
                    },
                ));
            }

            let id = data
                .attribute("id")
                .ok_or_else(|| {
                    located(
                        &data,
                        label.diagnostic_label,
                        ValidationError::MissingAttribute {
                            element: "Observer monitor <data>".into(),
                            attr: "id".into(),
                        },
                    )
                })?
                .to_string();

            let enter_expr = sce_attr(&data, "enter").ok_or_else(|| {
                located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: format!("Monitor '{id}'"),
                        attr: "sce:enter".into(),
                    },
                )
            })?;

            let leave_expr = sce_attr(&data, "leave");

            let on_enter = sce_attr(&data, "on-enter").ok_or_else(|| {
                located(
                    &data,
                    label.diagnostic_label,
                    ValidationError::MissingAttribute {
                        element: format!("Monitor '{id}'"),
                        attr: "sce:on-enter".into(),
                    },
                )
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
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Observer,
                what: "input field".into(),
            },
        ));
    }
    if monitors.is_empty() {
        return Err(located(
            &datamodel,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Observer,
                what: "monitor definition".into(),
            },
        ));
    }

    Ok(ObserverModel {
        name: label.identifier.to_string(),
        inputs,
        monitors,
        event_domain,
    })
}

// ── Algorithm parsing (RFC §5.A) ──────────────────────────────

fn parse_algorithm(
    root: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<AlgorithmModel, Located<ForgeError>> {
    let signature_node = find_sce_child(root, "signature").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Algorithm,
                element: "sce:signature".into(),
            },
        )
    })?;
    let signature = parse_algorithm_signature(&signature_node, label.diagnostic_label)?;

    let mut consts = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() == Some(SCE_NAMESPACE)
            && child.tag_name().name() == "const"
        {
            consts.push(parse_algorithm_const(&child, label.diagnostic_label)?);
        }
    }

    let body_node = find_sce_child(root, "body").ok_or_else(|| {
        located(
            root,
            label.diagnostic_label,
            ValidationError::MissingElement {
                kind: ForgeKind::Algorithm,
                element: "sce:body".into(),
            },
        )
    })?;
    let body = parse_algorithm_body(&body_node, label.diagnostic_label)?;
    if body.is_empty() {
        return Err(located(
            &body_node,
            label.diagnostic_label,
            ValidationError::EmptyCollection {
                kind: ForgeKind::Algorithm,
                what: "body statement".into(),
            },
        ));
    }

    // RFC §5.A `algorithm/lvalue-unsupported`: assigning to a parameter
    // is forbidden in v1. Walk the parsed body once at parse time so
    // diagnostics anchor at the body element rather than at codegen.
    reject_param_assignment(&body, &signature, &body_node, label.diagnostic_label)?;

    // RFC §5.A `algorithm/return-missing`: when the signature declares
    // a non-void return type, the body's terminal statement must be
    // `<sce:return>`. v1 only checks the trivial last-statement form;
    // flow-sensitive path coverage lands with §5.F (Phase A4).
    if signature.return_type.is_some()
        && !matches!(body.last(), Some(AlgorithmStmt::Return { .. }))
    {
        return Err(located(
            &body_node,
            label.diagnostic_label,
            ValidationError::AlgorithmReturnMissing,
        ));
    }

    Ok(AlgorithmModel {
        name: label.identifier.to_string(),
        signature,
        consts,
        body,
    })
}

fn parse_algorithm_signature(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<AlgorithmSignature, Located<ForgeError>> {
    let mut params = Vec::new();
    let mut return_type: Option<SceType> = None;
    let mut seen_return = false;

    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        match child.tag_name().name() {
            "param" => {
                let name = child
                    .attribute("name")
                    .ok_or_else(|| {
                        located(
                            &child,
                            doc_name,
                            ValidationError::MissingAttribute {
                                element: "<sce:param>".into(),
                                attr: "name".into(),
                            },
                        )
                    })?
                    .to_string();
                let type_str = child.attribute("type").ok_or_else(|| {
                    located(
                        &child,
                        doc_name,
                        ValidationError::MissingAttribute {
                            element: format!("<sce:param name=\"{name}\">"),
                            attr: "type".into(),
                        },
                    )
                })?;
                let sce_type = SceType::from_attr(type_str).ok_or_else(|| {
                    located(
                        &child,
                        doc_name,
                        ValidationError::InvalidAttribute {
                            element: format!("<sce:param name=\"{name}\">"),
                            attr: "type".into(),
                            value: type_str.into(),
                            expected: "uint8, uint16, uint32, uint64, int8, int16, int32, int64, float32, float64, bool, string, bytes".into(),
                        },
                    )
                })?;
                params.push(AlgorithmParam { name, sce_type });
            }
            "return" => {
                if seen_return {
                    return Err(located(
                        &child,
                        doc_name,
                        ValidationError::DuplicateId {
                            kind: ForgeKind::Algorithm,
                            what: "return declaration".into(),
                            id: "sce:return".into(),
                        },
                    ));
                }
                seen_return = true;
                if let Some(type_str) = child.attribute("type") {
                    let sce_type = SceType::from_attr(type_str).ok_or_else(|| {
                        located(
                            &child,
                            doc_name,
                            ValidationError::InvalidAttribute {
                                element: "<sce:return>".into(),
                                attr: "type".into(),
                                value: type_str.into(),
                                expected: "uint8..uint64, int8..int64, float32, float64, bool, string, bytes".into(),
                            },
                        )
                    })?;
                    return_type = Some(sce_type);
                }
            }
            _ => {}
        }
    }

    Ok(AlgorithmSignature {
        params,
        return_type,
    })
}

fn parse_algorithm_const(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<AlgorithmConst, Located<ForgeError>> {
    let name = node
        .attribute("name")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "<sce:const>".into(),
                    attr: "name".into(),
                },
            )
        })?
        .to_string();
    let type_str = node.attribute("type").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("<sce:const name=\"{name}\">"),
                attr: "type".into(),
            },
        )
    })?;
    let sce_type = AlgorithmConstType::from_attr(type_str).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("<sce:const name=\"{name}\">"),
                attr: "type".into(),
                value: type_str.into(),
                expected: "scalar (uint8..uint64, int8..int64, float32, float64, bool, string) \
                           or array<elem, len> (RFC §5.F)"
                    .into(),
            },
        )
    })?;

    // RFC §5.F: `sce:compute-at="build"` is the only legal value for
    // the attribute. Anything else is a hard error so future values
    // (e.g. `link`/`runtime`) can be added without silently sliding
    // past the parser. `None` (attribute absent) is the v1 default.
    let compute_at_attr = sce_attr(node, "compute-at");
    let compute_at_build = match compute_at_attr.as_deref() {
        None => false,
        Some("build") => true,
        Some(other) => {
            return Err(located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:const name=\"{name}\">"),
                    attr: "sce:compute-at".into(),
                    value: other.into(),
                    expected: "build (RFC §5.F build-time const-fold)".into(),
                },
            ));
        }
    };

    let fold_node = node.children().find(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(SCE_NAMESPACE)
            && n.tag_name().name() == "fold"
    });
    let init_attr = node.attribute("init").map(str::to_string);

    let const_label = format!("<sce:const name=\"{name}\">");
    // RFC §5.F: a `<sce:const>` is either a scalar literal
    // (`init=`) or a build-time fold (`<sce:fold>` body with
    // `sce:compute-at="build"`). The two forms are mutually exclusive.
    match (init_attr, fold_node, compute_at_build) {
        (Some(_), Some(fold), _) => Err(located(
            &fold,
            doc_name,
            ValidationError::IncompatibleAttributes {
                element: const_label.clone(),
                detail: "init= attribute conflicts with <sce:fold> body — \
                         scalar consts use init=, build-time consts use <sce:fold>"
                    .into(),
            },
        )),
        (None, None, _) => Err(located(
            node,
            doc_name,
            ValidationError::RequireEither {
                element: const_label.clone(),
                alternatives: vec!["init= attribute".into(), "<sce:fold> body".into()],
            },
        )),
        (Some(_), None, true) => Err(located(
            node,
            doc_name,
            ValidationError::IncompatibleAttributes {
                element: const_label.clone(),
                detail: "init= attribute conflicts with sce:compute-at=\"build\" — \
                         scalar literal consts cannot declare compute-at; \
                         use <sce:fold> for build-time evaluation"
                    .into(),
            },
        )),
        (Some(init), None, false) => {
            // Scalar literal form must declare a scalar type — array
            // forms only exist for fold-bodies.
            if matches!(sce_type, AlgorithmConstType::Array { .. }) {
                return Err(located(
                    node,
                    doc_name,
                    ValidationError::IncompatibleAttributes {
                        element: const_label.clone(),
                        detail: format!(
                            "type=\"{type_str}\" requires <sce:fold> body — \
                             array<...> consts cannot use init= \
                             (RFC §5.F build-time const-fold)"
                        ),
                    },
                ));
            }
            Ok(AlgorithmConst {
                name,
                sce_type,
                init: Some(init),
                fold: None,
                compute_at_build: false,
            })
        }
        (None, Some(fold_node), compute_at) => {
            if !compute_at {
                return Err(located(
                    &fold_node,
                    doc_name,
                    ValidationError::RequireEither {
                        element: const_label.clone(),
                        alternatives: vec!["sce:compute-at=\"build\" with <sce:fold> body".into()],
                    },
                ));
            }
            // A fold body requires an `array<elem, len>` outer type;
            // the host interpreter (Phase A4-β) emits one element per
            // iteration into a fixed-length array.
            let (expected_elem, expected_len) = match &sce_type {
                AlgorithmConstType::Array { elem, len } => (elem.clone(), *len),
                AlgorithmConstType::Scalar(_) => {
                    return Err(located(
                        node,
                        doc_name,
                        ValidationError::IncompatibleAttributes {
                            element: const_label.clone(),
                            detail: format!(
                                "type=\"{type_str}\" is scalar but body is <sce:fold> — \
                                 fold-form consts require an array<elem, len> outer type"
                            ),
                        },
                    ));
                }
            };
            let fold = parse_fold_body(&fold_node, doc_name, &expected_elem, expected_len)?;
            Ok(AlgorithmConst {
                name,
                sce_type,
                init: None,
                fold: Some(fold),
                compute_at_build: true,
            })
        }
    }
}

/// Parse an `<sce:fold>` element. Validates structure (range, iter
/// var, elem-type, terminating `<sce:yield>`) and that the outer
/// const's `array<elem, len>` matches the fold's declared `elem-type`
/// and the implied length (`range_end - range_start`).
fn parse_fold_body(
    node: &roxmltree::Node,
    doc_name: &str,
    expected_elem: &SceType,
    expected_len: u32,
) -> Result<FoldBody, Located<ForgeError>> {
    let range_str = require_attr(node, "range", "<sce:fold>", doc_name)?;
    let (range_start, range_end) = parse_fold_range(&range_str, node, doc_name)?;
    let iter_var = require_attr(node, "as", "<sce:fold>", doc_name)?;
    let elem_type_str = require_attr(node, "elem-type", "<sce:fold>", doc_name)?;
    let elem_type = parse_scetype_with_aliases_or_err(&elem_type_str, node, doc_name)?;

    if &elem_type != expected_elem {
        return Err(located(
            node,
            doc_name,
            ValidationError::IncompatibleAttributes {
                element: "<sce:fold>".into(),
                detail: format!(
                    "elem-type=\"{elem_type_str}\" does not match outer const's \
                     array element ({expected_elem:?}) — \
                     fold's elem-type must match the surrounding const's element type"
                ),
            },
        ));
    }

    let actual_len = range_end.saturating_sub(range_start);
    if actual_len != expected_len {
        return Err(located(
            node,
            doc_name,
            ValidationError::CountMismatch {
                kind: ForgeKind::Algorithm,
                detail: format!(
                    "<sce:fold> range produces {actual_len} elements but \
                     outer const declares array length {expected_len}"
                ),
            },
        ));
    }

    // Walk children: every sce:* element except the trailing
    // <sce:yield/> reuses the algorithm-statement vocabulary.
    let mut body: Vec<AlgorithmStmt> = Vec::new();
    let mut yield_expr: Option<String> = None;
    for child in node.children().filter(|c| c.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        let local = child.tag_name().name();
        if local == "yield" {
            if yield_expr.is_some() {
                return Err(located(
                    &child,
                    doc_name,
                    ValidationError::SingletonViolation {
                        kind: ForgeKind::Algorithm,
                        attr: "<sce:yield> in <sce:fold>".into(),
                    },
                ));
            }
            yield_expr = Some(require_attr(&child, "expr", "<sce:yield>", doc_name)?);
            continue;
        }
        if yield_expr.is_some() {
            return Err(located(
                &child,
                doc_name,
                ValidationError::UnsupportedKind(format!(
                    "<sce:{local}> after <sce:yield> in <sce:fold>; \
                     yield must be the terminal element"
                )),
            ));
        }
        body.push(parse_algorithm_stmt(&child, doc_name)?);
    }
    let yield_expr = yield_expr.ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::MissingElement {
                kind: ForgeKind::Algorithm,
                element: "<sce:yield> as terminal child of <sce:fold>".into(),
            },
        )
    })?;

    Ok(FoldBody {
        range_start,
        range_end,
        iter_var,
        elem_type,
        body,
        yield_expr,
    })
}

/// Parse `<sce:fold range="START..END">`. RFC §5.F worked example
/// uses the exclusive `0..256` form; that's the only shape accepted.
/// Negative starts and inclusive `..=` form are deferred until a
/// fixture actually demands them.
fn parse_fold_range(
    s: &str,
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<(u32, u32), Located<ForgeError>> {
    let s = s.trim();
    let (lo, hi) = s.split_once("..").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:fold>".into(),
                attr: "range".into(),
                value: s.into(),
                expected: "START..END (exclusive upper bound, RFC §5.F)".into(),
            },
        )
    })?;
    let lo: u32 = lo.trim().parse().map_err(|_| {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:fold>".into(),
                attr: "range".into(),
                value: s.into(),
                expected: "START..END with non-negative u32 endpoints".into(),
            },
        )
    })?;
    let hi: u32 = hi.trim().parse().map_err(|_| {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:fold>".into(),
                attr: "range".into(),
                value: s.into(),
                expected: "START..END with non-negative u32 endpoints".into(),
            },
        )
    })?;
    if hi < lo {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:fold>".into(),
                attr: "range".into(),
                value: s.into(),
                expected: "START..END with END >= START".into(),
            },
        ));
    }
    Ok((lo, hi))
}

fn parse_scetype_with_aliases_or_err(
    s: &str,
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<SceType, Located<ForgeError>> {
    crate::forge::model::AlgorithmConstType::from_attr(s)
        .and_then(|t| match t {
            crate::forge::model::AlgorithmConstType::Scalar(s) => Some(s),
            _ => None,
        })
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: "<sce:fold>".into(),
                    attr: "elem-type".into(),
                    value: s.into(),
                    expected: "scalar SceType (uint8..uint64, int8..int64, float32, float64)"
                        .into(),
                },
            )
        })
}

fn parse_algorithm_body(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<AlgorithmStmt>, Located<ForgeError>> {
    let mut stmts = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        stmts.push(parse_algorithm_stmt(&child, doc_name)?);
    }
    Ok(stmts)
}

fn parse_algorithm_stmt(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<AlgorithmStmt, Located<ForgeError>> {
    let local = node.tag_name().name();
    match local {
        "var" => {
            let name = require_attr(node, "name", "<sce:var>", doc_name)?;
            let type_str = require_attr(node, "type", "<sce:var>", doc_name)?;
            let sce_type = SceType::from_attr(&type_str).ok_or_else(|| {
                located(
                    node,
                    doc_name,
                    ValidationError::InvalidAttribute {
                        element: format!("<sce:var name=\"{name}\">"),
                        attr: "type".into(),
                        value: type_str.clone(),
                        expected: "uint8..uint64, int8..int64, float32, float64, bool, string, bytes".into(),
                    },
                )
            })?;
            let init = require_attr(node, "init", "<sce:var>", doc_name)?;
            Ok(AlgorithmStmt::Var {
                name,
                sce_type,
                init,
            })
        }
        "assign" => {
            let target = require_attr(node, "target", "<sce:assign>", doc_name)?;
            let expr = require_attr(node, "expr", "<sce:assign>", doc_name)?;
            Ok(AlgorithmStmt::Assign { target, expr })
        }
        "if" => {
            let cond = require_attr(node, "cond", "<sce:if>", doc_name)?;
            let mut then_body = Vec::new();
            let mut else_body: Option<Vec<AlgorithmStmt>> = None;
            for child in node.children().filter(|n| n.is_element()) {
                if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
                    continue;
                }
                if child.tag_name().name() == "else" {
                    if else_body.is_some() {
                        return Err(located(
                            &child,
                            doc_name,
                            ValidationError::DuplicateId {
                                kind: ForgeKind::Algorithm,
                                what: "else branch".into(),
                                id: "sce:else".into(),
                            },
                        ));
                    }
                    let mut else_stmts = Vec::new();
                    for c in child.children().filter(|n| n.is_element()) {
                        if c.tag_name().namespace() == Some(SCE_NAMESPACE) {
                            else_stmts.push(parse_algorithm_stmt(&c, doc_name)?);
                        }
                    }
                    else_body = Some(else_stmts);
                } else {
                    then_body.push(parse_algorithm_stmt(&child, doc_name)?);
                }
            }
            Ok(AlgorithmStmt::If {
                cond,
                then_body,
                else_body,
            })
        }
        "while" => {
            let cond = require_attr(node, "cond", "<sce:while>", doc_name)?;
            let max_iter = sce_attr(node, "max-iter").and_then(|s| parse_int(&s));
            let body = parse_algorithm_body(node, doc_name)?;
            Ok(AlgorithmStmt::While {
                cond,
                body,
                max_iter,
            })
        }
        "foreach" => {
            let item = require_attr(node, "item", "<sce:foreach>", doc_name)?;
            let source = require_attr(node, "in", "<sce:foreach>", doc_name)?;
            let body = parse_algorithm_body(node, doc_name)?;
            Ok(AlgorithmStmt::Foreach { item, source, body })
        }
        "return" => {
            let expr = node.attribute("expr").map(|s| s.to_string());
            Ok(AlgorithmStmt::Return { expr })
        }
        "call" => {
            let target = require_attr(node, "target", "<sce:call>", doc_name)?;
            let args = node
                .attribute("args")
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            Ok(AlgorithmStmt::Call { target, args })
        }
        other => Err(located(
            node,
            doc_name,
            ValidationError::UnsupportedKind(format!("<sce:{other}> in algorithm body")),
        )),
    }
}

/// RFC §5.A `algorithm/lvalue-unsupported`: parameters are read-only
/// in v1. Walks the body recursively. Anchors at `body_node` because
/// the offending `<sce:assign>` may be deeply nested; the body is the
/// nearest container element the diagnostic can point to without
/// re-threading nodes through the IR.
fn reject_param_assignment(
    stmts: &[AlgorithmStmt],
    sig: &AlgorithmSignature,
    body_node: &roxmltree::Node,
    doc_name: &str,
) -> Result<(), Located<ForgeError>> {
    for s in stmts {
        match s {
            AlgorithmStmt::Assign { target, .. } => {
                let head = target.split(['.', '[']).next().unwrap_or(target).trim();
                if sig.params.iter().any(|p| p.name == head) {
                    return Err(located(
                        body_node,
                        doc_name,
                        ValidationError::AlgorithmLvalueUnsupported {
                            target: target.clone(),
                            restriction: "algorithm parameters are read-only in v1".into(),
                        },
                    ));
                }
            }
            AlgorithmStmt::If {
                then_body,
                else_body,
                ..
            } => {
                reject_param_assignment(then_body, sig, body_node, doc_name)?;
                if let Some(eb) = else_body {
                    reject_param_assignment(eb, sig, body_node, doc_name)?;
                }
            }
            AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                reject_param_assignment(body, sig, body_node, doc_name)?;
            }
            AlgorithmStmt::Var { .. }
            | AlgorithmStmt::Return { .. }
            | AlgorithmStmt::Call { .. } => {}
        }
    }
    Ok(())
}

fn require_attr(
    node: &roxmltree::Node,
    attr: &str,
    element: &str,
    doc_name: &str,
) -> Result<String, Located<ForgeError>> {
    node.attribute(attr)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: element.into(),
                    attr: attr.into(),
                },
            )
        })
}

fn find_sce_child<'a>(
    node: &'a roxmltree::Node,
    local: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(SCE_NAMESPACE)
            && n.tag_name().name() == local
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
fn parse_imports(
    root: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<ForgeImport>, Located<ForgeError>> {
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
            .ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: "<sce:import>".into(),
                        attr: "src".into(),
                    },
                )
            })?
            .to_string();

        let kind_str = child.attribute("kind").ok_or_else(|| {
            located(
                &child,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "<sce:import>".into(),
                    attr: "kind".into(),
                },
            )
        })?;
        let kind = ForgeKind::from_attr(kind_str).ok_or_else(|| {
            located(
                &child,
                doc_name,
                ValidationError::UnsupportedKind(kind_str.to_string()),
            )
        })?;
        if !kind.is_supported() {
            return Err(located(
                &child,
                doc_name,
                ValidationError::UnsupportedKind(kind.to_string()),
            ));
        }
        if kind == ForgeKind::Statechart {
            return Err(located(
                &child,
                doc_name,
                ValidationError::WrongPipeline {
                    kind: ForgeKind::Statechart,
                },
            ));
        }

        let alias = child
            .attribute("as")
            .ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::MissingAttribute {
                        element: "<sce:import>".into(),
                        attr: "as".into(),
                    },
                )
            })?
            .to_string();

        if !aliases.insert(alias.clone()) {
            return Err(located(
                &child,
                doc_name,
                ValidationError::DuplicateId {
                    kind,
                    what: "alias".into(),
                    id: alias,
                },
            ));
        }

        let line = Some(child.document().text_pos_at(child.range().start).row);
        imports.push(ForgeImport {
            src,
            kind,
            alias,
            line,
        });
    }

    Ok(imports)
}

/// Extract only the import list from a forge SCXML (lightweight — no model parse).
/// Used by the manifest scanner to build dependency graphs.
///
/// `doc_name` is passed through so per-file import-validation errors
/// carry the correct filename even in the manifest-scanning path,
/// where each scanned file is processed separately.
pub fn parse_imports_only(
    content: &str,
    doc_name: &str,
) -> Result<Vec<ForgeImport>, Located<ForgeError>> {
    let doc = roxmltree::Document::parse(content).map_err(|e| {
        Located::new(
            XmlError::Parse(e.to_string()).into(),
            doc_name,
            None,
            None,
        )
    })?;
    let root = doc.root_element();
    parse_imports(&root, doc_name)
}

// ── Shared helpers ─────────────────────────────────────────────

/// Read an `sce:xxx` attribute from a node (namespace-qualified).
fn sce_attr(node: &roxmltree::Node, local_name: &str) -> Option<String> {
    node.attribute((SCE_NAMESPACE, local_name))
        .map(|s| s.to_string())
}

/// Parse `<sce:entry key="..." value="..."/>` children from a node.
///
/// Takes `doc_name` so each raise can anchor at the offending
/// `<sce:entry>` child rather than at the calling `<data>` parent —
/// the difference an agent sees as "row 12 of the lookup table" vs.
/// "somewhere in the datamodel".
///
/// Duplicate-key detection runs here (with the entry node still in
/// scope) rather than in the caller — both miss policies require
/// distinct keys to make the lookup deterministic, and anchoring
/// the diagnostic at the duplicating entry beats anchoring at the
/// surrounding `<datamodel>`.
fn parse_sce_entries(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<Vec<LookupEntry>, Located<ForgeError>> {
    let mut entries = Vec::new();
    let mut seen_keys = std::collections::BTreeSet::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "entry" && child.tag_name().namespace() == Some(SCE_NAMESPACE) {
            let key = child
                .attribute("key")
                .ok_or_else(|| {
                    located(
                        &child,
                        doc_name,
                        ValidationError::MissingAttribute {
                            element: "<sce:entry>".into(),
                            attr: "key".into(),
                        },
                    )
                })?
                .to_string();
            let value = child
                .attribute("value")
                .ok_or_else(|| {
                    located(
                        &child,
                        doc_name,
                        ValidationError::MissingAttribute {
                            element: "<sce:entry>".into(),
                            attr: "value".into(),
                        },
                    )
                })?
                .to_string();
            if !seen_keys.insert(key.clone()) {
                return Err(located(
                    &child,
                    doc_name,
                    ValidationError::DuplicateId {
                        kind: ForgeKind::Lookup,
                        what: "key".into(),
                        id: key,
                    },
                ));
            }
            entries.push(LookupEntry { key, value });
        }
    }
    Ok(entries)
}

/// Parse a typed forge field from a <data> element.
///
/// `doc_name` is threaded through so the helper can raise
/// `Located<ForgeError>` itself — keeps the call-site wrap-plumbing
/// uniform with every other kind parser instead of scattering
/// `.map_err(|e| located(&data, label.diagnostic_label, e))?` across the module.
fn parse_forge_field(
    data: &roxmltree::Node,
    doc_name: &str,
) -> Result<ForgeField, Located<ForgeError>> {
    let id = data
        .attribute("id")
        .ok_or_else(|| {
            located(
                data,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "Forge <data> field".into(),
                    attr: "id".into(),
                },
            )
        })?
        .to_string();

    let type_str = sce_attr(data, "type").ok_or_else(|| {
        located(
            data,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("Field '{id}'"),
                attr: "sce:type".into(),
            },
        )
    })?;
    let sce_type = SceType::from_attr(&type_str).ok_or_else(|| {
        located(
            data,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("field '{id}'"),
                attr: "sce:type".into(),
                value: type_str.clone(),
                expected: "uint8, uint16, uint32, int8, int16, int32, float32, float64, bool, string, bytes".into(),
            },
        )
    })?;

    let dir_str = sce_attr(data, "direction").ok_or_else(|| {
        located(
            data,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("Field '{id}'"),
                attr: "sce:direction".into(),
            },
        )
    })?;
    let direction = Direction::from_attr(&dir_str).ok_or_else(|| {
        located(
            data,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("field '{id}'"),
                attr: "sce:direction".into(),
                value: dir_str.clone(),
                expected: "in, out, internal".into(),
            },
        )
    })?;

    let expr = data.attribute("expr").map(|s| s.to_string());
    let unit = sce_attr(data, "unit");
    // RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B1: optional cap
    // on bytes-typed slots. Parsed for every field; the validator pass
    // (RFC §7) decides whether to flag it on a non-bytes field.
    let max_size = sce_attr(data, "max-size").and_then(|s| parse_int(&s));

    Ok(ForgeField {
        id,
        sce_type,
        direction,
        expr,
        unit,
        max_size,
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

/// Parse a u64 integer from a string (supports 0x hex prefix). Used by
/// RFC §5.B variant arm `value=` attributes which must hold any tag
/// width up to uint64.
fn parse_int_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}
