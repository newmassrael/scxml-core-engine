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
    // RFC §5.B "Test vector": v1 supports algorithm kind (B2) and
    // codec kind (B5-θ). Reject `<sce:test-vector>` elements
    // declared under any other kind here so the rejection anchors at
    // the offending element rather than at codegen time.
    if !matches!(kind, ForgeKind::Algorithm | ForgeKind::Codec) {
        if let Some(tv_node) = find_sce_child(root, "test-vector") {
            return Err(located(
                &tv_node,
                label.diagnostic_label,
                ValidationError::TestVectorUnsupportedKind {
                    name: label.identifier.to_string(),
                    kind,
                },
            ));
        }
    }
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
    // <sce:repeat> containers (RFC §5.B B2) sit alongside; their
    // bit_size = Repeat carries the count_ref + body alias for the
    // streaming codec to iterate the imported codec's encode/decode.
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
            "repeat" => {
                fields.push(parse_codec_repeat_from_node(&child, label.diagnostic_label)?);
            }
            "tlv-chain" => {
                fields.push(parse_codec_tlv_chain_from_node(&child, label)?);
            }
            _ => {}
        }
    }

    // RFC §5.B B5-α: zero-field codecs are accepted (empty-body messages
    // like Zenoh's KeepAlive sit at the wire-protocol level as a
    // declared-but-empty body keyed by the surrounding header byte).
    // Downstream validators walk the field list and tolerate
    // emptiness. A codec with `<sce:variant>` cannot be empty
    // because the variant's `tag` must resolve to a field; that
    // diagnostic surfaces from `parse_codec_variant` instead, with
    // a precise repair hint.

    // RFC §5.B B5-γ parent-flags dependency — codec-level
    // `<sce:requires-parent-flags carrier="X">` block declaring
    // the codec's body fields read flags from a parent codec's
    // flags carrier (Zenoh upstream pattern: `_z_init_decode(..,
    // uint8_t header)`). Cross-codec layout match against the
    // actual parent codec is deferred to the variant arm wire-up
    // validator (codegen-time) per `codec/parent-flag-mismatch`.
    let requires_parent_flags = parse_requires_parent_flags(root, label.diagnostic_label)?;

    // RFC §5.B B1-δ + B5-γ present-if validation — every gated
    // field's predicate must reference either a flags-bearing
    // carrier declared earlier (Local scope) or a flag declared
    // in the codec's `<sce:requires-parent-flags>` block (Parent
    // scope). Forward references and unknown Local carriers split
    // into distinct diagnostics (`codec/present-if-refs-later-field`
    // for the ordering case so the author gets a precise repair
    // hint; `validation/invalid-attribute` for missing carrier or
    // missing flag, since both reduce to "fix the attribute text").
    // Parent-scope predicates check flag existence in the declared
    // requires-parent-flags block; cross-codec match defers to
    // codegen-time validator.
    validate_codec_present_if_predicates(
        &fields,
        requires_parent_flags.as_ref(),
        label,
        &datamodel,
    )?;

    // RFC §5.B B2 repeat validation — every <sce:repeat sce:count="X"/>
    // must reference a sibling integer field declared earlier (so the
    // streaming decoder has already decoded N before reading N
    // elements). Forward / unknown count target → typed
    // `codec/repeat-count-refs-later-field`; non-integer count target
    // reuses the generic `validation/invalid-attribute`.
    validate_codec_repeat_count_refs(&fields, label, &datamodel)?;

    // RFC §5.B B5-μ — co-gating constraint for repeat-with-present-if
    // (Wire RFC Phase B X1). When a `<sce:repeat sce:count="X"
    // sce:present-if="P"/>` field is gated, the count source field
    // `X` MUST carry the IDENTICAL `sce:present-if="P"` predicate
    // (same scope, same field_id, same flag_name, same negate). The
    // semantic invariant: when the gate fires off, the count byte(s)
    // are absent from the wire too — co-gating is the only authoring
    // shape where the streaming decoder can safely read the count
    // before emitting the repeat block. Folded into
    // `validation/invalid-attribute` so the repair (align both
    // attribute texts) reads off the diagnostic.
    validate_codec_repeat_present_if_co_gating(&fields, label, &datamodel)?;

    // RFC §5.B B3 DMA alignment validation — `sce:dma-burst-align="N"`
    // requires (a) the field's authored `sce:byte` be divisible by N,
    // and (b) every preceding field is Fixed bit-size (so post-padding
    // layout is statically computable). Both gates fold into
    // `codec/dma-alignment-unsatisfiable` so the author sees one
    // diagnostic naming the offending field.
    validate_codec_dma_alignment(&fields, label, &datamodel)?;

    // RFC §5.B B5-κ Surface L — `sce:length-field="<carrier>.<flag>"`
    // dotted-path form (length value sourced from a multi-bit flag
    // subfield inside a flags-bearing carrier, mirroring the B1-δ
    // present-if dotted-path grammar). Validation: carrier must be
    // declared earlier in the same codec, must be flags-bearing, must
    // contain a flag of that name, AND the flag must be multi-bit
    // (width > 1) since a single-bit flag would only ever carry
    // value 0 or 1 — that's the present-if grammar's purpose.
    // Plain (non-dotted) length-field is left untouched here (existing
    // codegen-time lookup).
    validate_codec_length_field_refs(&fields, label, &datamodel)?;

    // RFC §5.B variant primitive (B1-β): optional <sce:variant> suffix
    // under <datamodel>. Resolves the tag field reference against the
    // codec's own field list; arm body aliases (resolved against
    // <sce:import> aliases) are validated downstream by the codegen
    // step which has the import set.
    let variant = parse_codec_variant(&datamodel, &fields, label)?;

    // RFC §5.B B5-θ inline test vectors. Parsed against the field list
    // so each `<sce:decoded field="..." value|hex|string="..."/>` row
    // resolves to a typed value matching the field's `SceType`. Trunk
    // accepts plain (non-variant, non-TLV-chain, non-parent-flags)
    // codecs only — the per-language sidecar emitter rejects the
    // out-of-trunk shapes through `render_codec_test_vector_sidecar`'s
    // gate so the parser surface stays uniform across closures.
    let test_vectors = parse_codec_test_vectors(root, &fields, label)?;

    Ok(CodecModel {
        name: label.identifier.to_string(),
        default_endian,
        input_length,
        fields,
        variant,
        requires_parent_flags,
        test_vectors,
    })
}

/// RFC §5.B B5-γ — parse the optional codec-level
/// `<sce:requires-parent-flags carrier="X">` element with `<sce:flag
/// name="N" bit="B"/>` children. Returns `None` when the element is
/// absent. The carrier attribute is required (names the parent
/// codec's flags-carrier field id); each child `<sce:flag>` declares
/// a named bit in the parent's carrier. v1 single-bit only (no
/// `width` attribute) — multi-bit parent-flag access defers to a
/// later B-stage when a reachable consumer surfaces.
///
/// Cross-codec layout match against the parent codec's
/// `<sce:flags id="<carrier>">` happens at variant arm wire-up time
/// (codegen-time validator) per `codec/parent-flag-mismatch`. Here
/// we only validate intra-element shape (carrier non-empty, flag
/// name uniqueness, bit within u8 range — v1 fixes parent flag
/// carrier type at uint8 per Zenoh transport pattern).
fn parse_requires_parent_flags(
    codec_root: &roxmltree::Node,
    doc_name: &str,
) -> Result<Option<RequiresParentFlags>, Located<ForgeError>> {
    let block = codec_root.children().find(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(SCE_NAMESPACE)
            && n.tag_name().name() == "requires-parent-flags"
    });
    let block = match block {
        Some(b) => b,
        None => return Ok(None),
    };
    // `carrier` is an unqualified attribute (matches the
    // `<sce:arm value/type>` / `<sce:flag name/bit>` convention used
    // throughout the codec sub-elements; only globally-shared attrs
    // like `sce:byte` keep the namespace prefix).
    let carrier = block
        .attribute("carrier")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if carrier.is_empty() {
        return Err(located(
            &block,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:requires-parent-flags>".into(),
                attr: "carrier".into(),
                value: String::new(),
                expected: "non-empty parent codec flags-carrier field id \
                           (e.g. carrier=\"header\")"
                    .into(),
            },
        ));
    }
    let mut flags: Vec<FlagDef> = Vec::new();
    for child in block.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        if child.tag_name().name() != "flag" {
            continue;
        }
        let name = child
            .attribute("name")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: "<sce:flag> in <sce:requires-parent-flags>".into(),
                    attr: "name".into(),
                    value: String::new(),
                    expected: "non-empty flag name (matches a flag declared on \
                               the parent codec's <sce:flags> carrier)"
                        .into(),
                },
            ));
        }
        let bit_str = child
            .attribute("bit")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let bit: u32 = parse_int(&bit_str).ok_or_else(|| {
            located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag name=\"{}\">", name),
                    attr: "bit".into(),
                    value: bit_str.clone(),
                    expected: "non-negative integer bit position within the \
                               parent carrier (v1 fixes parent type at uint8: \
                               bit ∈ [0, 7])"
                        .into(),
                },
            )
        })?;
        if bit >= 8 {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag name=\"{}\">", name),
                    attr: "bit".into(),
                    value: bit_str,
                    expected: "bit position must lie within the parent flags \
                               carrier (v1 fixes parent type at uint8: \
                               bit ∈ [0, 7])"
                        .into(),
                },
            ));
        }
        if flags.iter().any(|f| f.name == name) {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: "<sce:requires-parent-flags>".into(),
                    attr: "name".into(),
                    value: name.clone(),
                    expected: "unique flag name within the requires-parent-flags \
                               block"
                        .into(),
                },
            ));
        }
        flags.push(FlagDef {
            name,
            bit,
            width: 1,
        });
    }
    if flags.is_empty() {
        return Err(located(
            &block,
            doc_name,
            ValidationError::InvalidAttribute {
                element: "<sce:requires-parent-flags>".into(),
                attr: "child elements".into(),
                value: String::new(),
                expected: "at least one <sce:flag name=\"N\" bit=\"B\"/> child \
                           (an empty parent-flags block has no purpose)"
                    .into(),
            },
        ));
    }
    Ok(Some(RequiresParentFlags { carrier, flags }))
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
/// RFC §5.B B1-δ present-if cross-field validation. Walks the
/// declared field list in source order; each field's `present_if`
/// predicate must reference a *previously-declared* sibling field
/// that carries the `<sce:flags>` shape (so the streaming decoder
/// can read the flag bit before reaching the gated field). Forward
/// references — predicate target declared after the consumer, or
/// not declared at all — emit
/// `codec/present-if-refs-later-field`. Carrier-shape and flag-name
/// mismatches reuse the generic `validation/invalid-attribute`
/// because the repair is still "fix the attribute text".
fn validate_codec_present_if_predicates(
    fields: &[CodecField],
    requires_parent_flags: Option<&RequiresParentFlags>,
    label: DocumentLabel<'_>,
    datamodel: &roxmltree::Node,
) -> Result<(), Located<ForgeError>> {
    use std::collections::BTreeMap;
    let mut by_id_so_far: BTreeMap<&str, &CodecField> = BTreeMap::new();
    for field in fields {
        if let Some(predicate) = &field.present_if {
            // RFC §5.B B5-γ: parent-scope predicates resolve against
            // the codec's `<sce:requires-parent-flags>` block, not
            // against sibling fields. The cross-codec match against
            // the actual parent codec's `<sce:flags id="<carrier>">`
            // happens at variant arm wire-up time (Task 5 cross-codec
            // validator); here we only confirm the codec authored a
            // declared parent contract and the flag name appears in
            // it. Mismatch reuses `validation/invalid-attribute`
            // because the repair is "fix the attribute text or add
            // the flag to the requires-parent-flags block".
            if predicate.scope == PresentIfScope::Parent {
                let parent = match requires_parent_flags {
                    Some(p) => p,
                    None => {
                        return Err(located(
                            datamodel,
                            label.diagnostic_label,
                            ValidationError::InvalidAttribute {
                                element: format!(
                                    "field '{}' in codec '{}'",
                                    field.id, label.identifier
                                ),
                                attr: "sce:present-if".into(),
                                value: format!("parent.{}", predicate.flag_name),
                                expected:
                                    "codec must declare \
                                     <sce:requires-parent-flags carrier=\"...\"> \
                                     before using a 'parent.<flag>' predicate"
                                        .into(),
                            },
                        ));
                    }
                };
                if !parent.flags.iter().any(|f| f.name == predicate.flag_name) {
                    let known: Vec<&str> =
                        parent.flags.iter().map(|f| f.name.as_str()).collect();
                    return Err(located(
                        datamodel,
                        label.diagnostic_label,
                        ValidationError::InvalidAttribute {
                            element: format!(
                                "field '{}' in codec '{}'",
                                field.id, label.identifier
                            ),
                            attr: "sce:present-if".into(),
                            value: format!("parent.{}", predicate.flag_name),
                            expected: format!(
                                "flag name must be declared in \
                                 <sce:requires-parent-flags carrier=\"{}\">: \
                                 known flags = [{}]",
                                parent.carrier,
                                known.join(", ")
                            ),
                        },
                    ));
                }
                // Parent predicate validated; advance the local
                // by_id_so_far walker so subsequent fields still
                // see this gated field's id (matches Local-scope
                // semantics — gated fields ARE field-id-visible).
                by_id_so_far.insert(field.id.as_str(), field);
                continue;
            }
            match by_id_so_far.get(predicate.field_id.as_str()) {
                None => {
                    return Err(located(
                        datamodel,
                        label.diagnostic_label,
                        ValidationError::CodecPresentIfRefsLaterField {
                            codec: label.identifier.to_string(),
                            field: field.id.clone(),
                            refers_to: predicate.field_id.clone(),
                        },
                    ));
                }
                Some(carrier) => {
                    if !carrier.is_flags_carrier() {
                        return Err(located(
                            datamodel,
                            label.diagnostic_label,
                            ValidationError::InvalidAttribute {
                                element: format!(
                                    "field '{}' in codec '{}'",
                                    field.id, label.identifier
                                ),
                                attr: "sce:present-if".into(),
                                value: format!(
                                    "{}.{}",
                                    predicate.field_id, predicate.flag_name
                                ),
                                expected: format!(
                                    "predicate LHS must reference a flags-bearing \
                                     carrier (declared via <sce:flags>); '{}' is \
                                     a plain field",
                                    predicate.field_id
                                ),
                            },
                        ));
                    }
                    if !carrier
                        .flags
                        .iter()
                        .any(|f| f.name == predicate.flag_name)
                    {
                        let known: Vec<&str> =
                            carrier.flags.iter().map(|f| f.name.as_str()).collect();
                        return Err(located(
                            datamodel,
                            label.diagnostic_label,
                            ValidationError::InvalidAttribute {
                                element: format!(
                                    "field '{}' in codec '{}'",
                                    field.id, label.identifier
                                ),
                                attr: "sce:present-if".into(),
                                value: format!(
                                    "{}.{}",
                                    predicate.field_id, predicate.flag_name
                                ),
                                expected: format!(
                                    "flag name must be declared on carrier \
                                     '{}': known flags = [{}]",
                                    predicate.field_id,
                                    known.join(", ")
                                ),
                            },
                        ));
                    }
                }
            }
        }
        by_id_so_far.insert(field.id.as_str(), field);
    }
    Ok(())
}

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
    let raw_tag = variant_node
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

    // RFC §5.B B5-β multi-bit-flag dispatch: `tag="<carrier>.<flag>"`
    // names a bit-range within a flags-bearing carrier; bare
    // `tag="<field>"` (B1-β whole-field form) keeps the original
    // semantics. Grammar mirrors B1-δ present-if predicate exactly so
    // authors learn one dotted-path convention.
    let (tag_field, tag_flag): (String, Option<String>) = match raw_tag.split_once('.') {
        Some((carrier, flag)) => {
            let carrier = carrier.trim();
            let flag = flag.trim();
            if carrier.is_empty() || flag.is_empty() {
                return Err(located(
                    &variant_node,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "<sce:variant>".into(),
                        attr: "tag".into(),
                        value: raw_tag.clone(),
                        expected: "either a bare field id (e.g. 'msg_id') for whole-field \
                                   dispatch, or a '<carrier>.<flag>' dotted path (e.g. \
                                   'header.mid') for multi-bit-flag dispatch — both halves \
                                   must be non-empty"
                            .into(),
                    },
                ));
            }
            (carrier.to_string(), Some(flag.to_string()))
        }
        None => (raw_tag.clone(), None),
    };

    // Resolve tag against the codec's own fields and capture its type
    // for arm-domain reasoning. The tag field MUST be unsigned-int
    // (uint8/uint16/uint32/uint64) because the arm `value=` matches a
    // wire-decoded unsigned scalar; signed / bytes / float tags have
    // no valid discriminator semantics.
    //
    // For the B5-β `<carrier>.<flag>` form, the carrier additionally
    // MUST be a `<sce:flags>`-bearing field (parser invariant: flags
    // carriers are always unsigned-int, so the unsigned check still
    // holds), and `flag` MUST name one of its `<sce:flag>` children.
    let tag_field_ref = match fields.iter().find(|f| f.id == tag_field) {
        Some(f) if f.sce_type.is_unsigned() => f,
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
    let tag_type = tag_field_ref.sce_type.clone();

    // B5-β: if the tag uses dotted form, the carrier must carry flags
    // and the named flag must exist. Width of the named flag determines
    // both the dispatch domain (1<<width) and the result-type used by
    // arm value literals downstream. Failures stay on
    // `validation/invalid-attribute` because the repair is still
    // attribute-text-level (mirrors B1-δ present-if's choice).
    let tag_flag_width: Option<u32> = match &tag_flag {
        Some(flag_name) => {
            if tag_field_ref.flags.is_empty() {
                return Err(located(
                    &variant_node,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "<sce:variant>".into(),
                        attr: "tag".into(),
                        value: format!("{tag_field}.{flag_name}"),
                        expected: format!(
                            "carrier '{tag_field}' must be authored as <sce:flags> with \
                             <sce:flag> children for the dotted-path form; '{tag_field}' \
                             is a plain field — either author it as <sce:flags> or use \
                             bare tag=\"{tag_field}\" for whole-field dispatch"
                        ),
                    },
                ));
            }
            match tag_field_ref.flags.iter().find(|f| f.name == *flag_name) {
                Some(flag_def) => Some(flag_def.width.max(1)),
                None => {
                    let available: Vec<String> =
                        tag_field_ref.flags.iter().map(|f| f.name.clone()).collect();
                    return Err(located(
                        &variant_node,
                        label.diagnostic_label,
                        ValidationError::InvalidAttribute {
                            element: "<sce:variant>".into(),
                            attr: "tag".into(),
                            value: format!("{tag_field}.{flag_name}"),
                            expected: format!(
                                "flag '{flag_name}' is not declared on carrier \
                                 '{tag_field}' — available flags: {}",
                                available.join(", ")
                            ),
                        },
                    ));
                }
            }
        }
        None => None,
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
    // uint64 always require a default. For the B5-β multi-bit-flag
    // dispatch form the domain shrinks to `1 << width` of the named
    // bit-range (e.g. width=5 ⇒ 32 values), which is always
    // practically enumerable since `<sce:flag>` width itself is bounded
    // by carrier_int_bit_width ≤ 64.
    if default_arm.is_none() {
        let domain_size: Option<u64> = match tag_flag_width {
            Some(width) => Some(1u64 << width),
            None => match tag_type {
                SceType::Uint8 => Some(256),
                SceType::Uint16 => Some(65_536),
                _ => None,
            },
        };
        let arm_count = arms.len();
        let exhaustive = match domain_size {
            Some(n) => (arm_count as u64) >= n,
            None => false,
        };
        if !exhaustive {
            // Surface the named bit-range in the diagnostic label so
            // authors immediately see they're authoring a sub-domain
            // (otherwise "tag 'header'" would mislead toward a 256-arm
            // exhaustiveness expectation when the actual domain is
            // 1<<width). The diagnostic's `tag_type` field stays the
            // carrier type for back-compat with the unreachable test.
            let display_tag = match &tag_flag {
                Some(flag_name) => format!("{tag_field}.{flag_name}"),
                None => tag_field.clone(),
            };
            return Err(located(
                &variant_node,
                label.diagnostic_label,
                ValidationError::CodecVariantArmUnreachable {
                    codec: label.identifier.to_string(),
                    tag_field: display_tag,
                    tag_type: format!("{tag_type:?}").to_lowercase(),
                    arm_count,
                    domain_size,
                },
            ));
        }
    }

    Ok(Some(CodecVariant {
        tag_field,
        tag_flag,
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

    // RFC §5.B B5-δ Surface F — `sce:length-arith="+1"|"-1"` arithmetic
    // offset on the length sibling's value. v1 grammar restricts to
    // `±1` (parser rejects 0 and `|x| > 1`); widening defers to a
    // reachable consumer. Standalone `length-arith` without
    // `length-field` is rejected (the offset has no source to apply to).
    let length_arith = match sce_attr(node, "length-arith") {
        None => None,
        Some(raw) => {
            let trimmed = raw.trim();
            // Accept `+1`, `-1`, `1`. Reject everything else.
            let n: i32 = match trimmed {
                "+1" | "1" => 1,
                "-1" => -1,
                _ => {
                    return Err(located(
                        node,
                        doc_name,
                        ValidationError::InvalidAttribute {
                            element: format!("Codec field '{id}'"),
                            attr: "sce:length-arith".into(),
                            value: raw.clone(),
                            expected: "+1 or -1 (v1 limits arithmetic offset to ±1; \
                                       widening defers to a reachable consumer)"
                                .into(),
                        },
                    ));
                }
            };
            if length_field.is_none() {
                return Err(located(
                    node,
                    doc_name,
                    ValidationError::InvalidAttribute {
                        element: format!("Codec field '{id}'"),
                        attr: "sce:length-arith".into(),
                        value: raw.clone(),
                        expected: "sce:length-arith requires sce:length-field \
                                   (the offset has no source to apply to)"
                            .into(),
                    },
                ));
            }
            if !matches!(bit_size, BitSize::LengthRef) {
                return Err(located(
                    node,
                    doc_name,
                    ValidationError::InvalidAttribute {
                        element: format!("Codec field '{id}'"),
                        attr: "sce:length-arith".into(),
                        value: raw.clone(),
                        expected: "sce:length-arith requires sce:bit-size=\"length-ref\" \
                                   (the offset adjusts the byte count read from the \
                                   referenced sibling)"
                            .into(),
                    },
                ));
            }
            Some(n)
        }
    };

    // RFC §5.B B1-δ present-if primitive — accept the attribute and
    // parse the v1 grammar `<field_id>.<flag_name>` here, but defer
    // the cross-field forward-reference and flags-carrier-existence
    // checks to the codec-level pass where the full field set is
    // known. This keeps `parse_codec_field_from_node` reusable for
    // both `<sce:field>` and `<sce:flags>` (where the carrier itself
    // can never carry a present-if attribute, and the v1 grammar
    // would error on an empty / malformed predicate at attribute
    // read time).
    let present_if = match sce_attr(node, "present-if") {
        None => None,
        Some(raw) => Some(parse_present_if_predicate(&raw, node, doc_name, &id)?),
    };

    // RFC §5.B B3 DMA alignment primitive — `sce:dma-burst-align="N"`
    // declares this field's encoded-buffer offset is constrained to an
    // N-byte boundary. v1 attribute-text-level validation: must parse
    // as positive integer and N must be a power of 2 (typical: 16 / 32
    // / 64; reject 0 / 3 / 5). Cross-field validation
    // (`codec/dma-alignment-unsatisfiable` — preceding field non-fixed)
    // runs in `validate_codec_dma_alignment` after the field list is
    // assembled.
    let dma_burst_align = match sce_attr(node, "dma-burst-align") {
        None => None,
        Some(raw) => {
            let n = parse_int(&raw).ok_or_else(|| {
                located(
                    node,
                    doc_name,
                    ValidationError::NumericParse {
                        element: format!("Codec field '{id}'"),
                        attr: "sce:dma-burst-align".into(),
                        value: raw.clone(),
                        detail: "expected positive integer".into(),
                    },
                )
            })?;
            if n == 0 || (n & (n - 1)) != 0 {
                return Err(located(
                    node,
                    doc_name,
                    ValidationError::InvalidAttribute {
                        element: format!("Codec field '{id}'"),
                        attr: "sce:dma-burst-align".into(),
                        value: raw.clone(),
                        expected: "positive power-of-2 integer (e.g. 16, 32, 64)".into(),
                    },
                ));
            }
            Some(n)
        }
    };

    // RFC §5.B B5-ζ Surface H — `sce:type="string"` v1 surface:
    // requires `sce:bit-size="length-ref"` (UTF-8 text is length-
    // prefixed; tail / fixed-bit / vle defer until a consumer
    // surfaces) AND cannot carry `sce:present-if` (gated String has
    // no v1 consumer; defers to the same condition). The generic
    // `validation/invalid-attribute` reports a precise repair hint
    // so authors see the legal combination upfront. Mirror zenoh-
    // pico `_z_string_encode/decode` (codec.c:324-343).
    if matches!(sce_type, SceType::String) && !matches!(bit_size, BitSize::LengthRef) {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("Codec field '{id}'"),
                attr: "sce:bit-size".into(),
                value: match &bit_size {
                    BitSize::Tail => "tail".into(),
                    BitSize::Fixed { bits } => format!("{bits}"),
                    BitSize::Vle { .. } => "vle".into(),
                    BitSize::Repeat { .. } => "<repeat>".into(),
                    BitSize::TlvChain { .. } => "<tlv-chain>".into(),
                    BitSize::LengthRef => unreachable!("guarded by outer match"),
                },
                expected: "sce:type=\"string\" requires sce:bit-size=\"length-ref\" \
                           (UTF-8 text is length-prefixed; tail / fixed-bit / vle \
                           shapes defer until a consumer surfaces)"
                    .into(),
            },
        ));
    }
    if matches!(sce_type, SceType::String) && present_if.is_some() {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("Codec field '{id}'"),
                attr: "sce:present-if".into(),
                value: "<predicate>".into(),
                expected: "sce:type=\"string\" cannot carry sce:present-if in v1 \
                           (gated String defers until a consumer surfaces; v1 String \
                           fields are always present)"
                    .into(),
            },
        ));
    }

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
        present_if,
        // RFC §5.B B2/B3: `<sce:field>` carriers never hold a body
        // alias — only the dedicated `<sce:repeat>` /
        // `<sce:tlv-chain>` elements set these. Plain fields keep
        // them None.
        repeat_body_alias: None,
        max_count: None,
        tlv_chain_body_alias: None,
        dma_burst_align,
        length_arith,
    })
}

/// RFC §5.B B1-δ + B5-γ + B5-λ present-if predicate grammar.
///
/// Four forms in v1:
///   - `<field_id>.<flag_name>` (B1-δ Local positive) — `field_id`
///     names a flags-bearing sibling field declared earlier in the
///     same codec; predicate fires when the bit is set.
///   - `!<field_id>.<flag_name>` (B5-λ Local negative) — same
///     carrier, predicate fires when the bit is *clear*.
///   - `parent.<flag_name>` (B5-γ Parent positive) — the literal
///     `parent` keyword references the codec's declared
///     `<sce:requires-parent-flags>` block; predicate fires when the
///     bit is set.
///   - `!parent.<flag_name>` (B5-λ Parent negative) — fires when
///     the parent flag bit is clear. Required for Zenoh OpenSyn body
///     where cookie is present iff parent.A is NOT set.
///
/// `field_id` is empty when scope = Parent (carrier is implicit).
///
/// Both halves match the SCE attribute name shape (alphanumeric +
/// `_`, non-empty). Conjunction (`flag1 && flag2`) and equality
/// (`field == value`) defer to a later B-stage.
fn parse_present_if_predicate(
    raw: &str,
    node: &roxmltree::Node,
    doc_name: &str,
    field_id: &str,
) -> Result<PresentIfPredicate, Located<ForgeError>> {
    let trimmed = raw.trim();
    let invalid = || {
        located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("field '{field_id}'"),
                attr: "sce:present-if".into(),
                value: raw.to_string(),
                expected: "one of '<field_id>.<flag_name>' / \
                           '!<field_id>.<flag_name>' / 'parent.<flag_name>' / \
                           '!parent.<flag_name>' (v1 grammar — optional '!' \
                           prefix for negation; both halves are non-empty \
                           identifiers; conjunction and equality defer to a \
                           later RFC §5.B stage)"
                    .into(),
            },
        )
    };
    // B5-λ: optional leading `!` for negation.
    let (negate, body) = match trimmed.strip_prefix('!') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, trimmed),
    };
    let (lhs, rhs) = body.split_once('.').ok_or_else(invalid)?;
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if lhs.is_empty() || rhs.is_empty() {
        return Err(invalid());
    }
    let is_ident = |s: &str| {
        s.chars().enumerate().all(|(i, c)| {
            if i == 0 {
                c.is_ascii_alphabetic() || c == '_'
            } else {
                c.is_ascii_alphanumeric() || c == '_'
            }
        })
    };
    if !is_ident(lhs) || !is_ident(rhs) {
        return Err(invalid());
    }
    // RFC §5.B B5-γ: `parent` is a reserved LHS keyword that
    // references the codec's declared `<sce:requires-parent-flags>`
    // block. The cross-codec validator confirms the carrier and
    // flag exist on the parent codec at variant arm wire-up time;
    // here we only record the scope so codegen can branch on it.
    if lhs == "parent" {
        Ok(PresentIfPredicate {
            scope: PresentIfScope::Parent,
            field_id: String::new(),
            flag_name: rhs.to_string(),
            negate,
        })
    } else {
        Ok(PresentIfPredicate {
            scope: PresentIfScope::Local,
            field_id: lhs.to_string(),
            flag_name: rhs.to_string(),
            negate,
        })
    }
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

    // Reject present-if on a flags carrier — a carrier that is itself
    // gated cannot serve as the LHS of another field's present-if
    // (the flag bit it carries is unreadable when the carrier is
    // absent). v1 forbids this composition; later stages can lift
    // the restriction when a reachable consumer surfaces.
    if field.present_if.is_some() {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("<sce:flags id='{}'>", field.id),
                attr: "sce:present-if".into(),
                value: "<predicate>".into(),
                expected: "<sce:flags> carriers cannot themselves be \
                           gated by present-if (the bit they carry would \
                           be unreadable when the carrier is absent)"
                    .into(),
            },
        ));
    }

    let bit_width = field.sce_type.int_bit_width().expect("unsigned ⇒ Some");

    let mut seen_names: std::collections::BTreeSet<String> = Default::default();
    // Track which bits within the carrier are already claimed so
    // overlapping bit-ranges (B5-α multi-bit) are rejected with a
    // precise repair hint.
    let mut occupied: u64 = 0;
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
        // RFC §5.B B5-α multi-bit accessor: optional `width="W"`
        // attribute (defaults to 1 for B1-γ single-bit back-compat).
        // `bit + width <= carrier_int_bit_width` so the named range
        // stays within the carrier's natural width.
        let width = match child.attribute("width") {
            None => 1u32,
            Some(s) => parse_int(s).ok_or_else(|| {
                located(
                    &child,
                    doc_name,
                    ValidationError::NumericParse {
                        element: format!("<sce:flag name='{name}'>"),
                        attr: "width".into(),
                        value: s.to_string(),
                        detail: "expected positive integer".into(),
                    },
                )
            })?,
        };
        if width == 0 {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag name='{name}'>"),
                    attr: "width".into(),
                    value: width.to_string(),
                    expected: "1..=carrier_bit_width".into(),
                },
            ));
        }
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
        if bit + width > bit_width {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag name='{name}'>"),
                    attr: "width".into(),
                    value: width.to_string(),
                    expected: format!(
                        "bit({bit}) + width <= {bit_width} (carrier is {bit_width}-bit)"
                    ),
                },
            ));
        }
        // Build the bit-range mask in the carrier's natural width and
        // reject if any bit overlaps a previously-declared sibling.
        let range_mask: u64 = ((1u64 << width) - 1) << bit;
        if occupied & range_mask != 0 {
            return Err(located(
                &child,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:flag name='{name}'>"),
                    attr: "bit".into(),
                    value: format!("{bit}..{}", bit + width),
                    expected: "bit-range disjoint from siblings in same <sce:flags>"
                        .into(),
                },
            ));
        }
        occupied |= range_mask;
        flag_defs.push(FlagDef { name, bit, width });
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

/// RFC §5.B B2 repeat primitive — parse `<sce:repeat id="..."
/// sce:type="<imported_alias>" sce:byte="N"
/// (sce:count="<id>" | sce:until-eof="true")
/// [sce:max-count="N"]/>`.
///
/// The element produces a `CodecField` whose `bit_size` is
/// [`BitSize::Repeat`] and whose `repeat_body_alias` names the
/// imported codec used for each element. Mutually exclusive
/// `sce:count` / `sce:until-eof` (exactly one required) drives the
/// streaming loop termination strategy.
///
/// `sce:type` does NOT round-trip through [`SceType`] — it carries
/// the imported codec alias verbatim (mirrors `<sce:arm type=...>`).
/// Resolution against `<sce:import>` aliases happens downstream at
/// codegen time when the import set is in scope.
fn parse_codec_repeat_from_node(
    node: &roxmltree::Node,
    doc_name: &str,
) -> Result<CodecField, Located<ForgeError>> {
    // `id` is unqualified on <sce:repeat> (matches <sce:flags id=...>
    // and <sce:variant tag=...> conventions for SCE-internal attrs).
    let id = node
        .attribute("id")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "<sce:repeat>".into(),
                    attr: "id".into(),
                },
            )
        })?
        .to_string();

    // `type` on <sce:repeat> is the imported codec alias, NOT a
    // primitive sce:sceType — kept unqualified to match <sce:arm
    // type=...>, distinct from the global qualified `sce:type` which
    // restricts to primitive sceType enum values. Stored as
    // `repeat_body_alias`; the parsed CodecField's `sce_type` is a
    // sentinel (SceType::Bytes) since the wire shape is "byte
    // sequence encoding N imported-codec elements" — the host
    // language type (Vec<T> / std::vector<T>) is computed by the
    // generator from the body alias.
    let body_alias = node
        .attribute("type")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: format!("<sce:repeat id='{id}'>"),
                    attr: "type".into(),
                },
            )
        })?
        .to_string();

    // `sce:byte` is qualified — same global attribute reused on
    // <sce:field> / <sce:flags> (consistent global usage).
    let byte_offset_str = sce_attr(node, "byte").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("<sce:repeat id='{id}'>"),
                attr: "sce:byte".into(),
            },
        )
    })?;
    let byte_offset = parse_int(&byte_offset_str).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::NumericParse {
                element: format!("<sce:repeat id='{id}'>"),
                attr: "sce:byte".into(),
                value: byte_offset_str.clone(),
                detail: "expected integer".into(),
            },
        )
    })?;

    // Mutually exclusive count source: exactly one of count /
    // until-eof. Both unqualified (SCE-internal). Both present or
    // both absent → reject with a diagnostic that names the legal
    // forms (the author can flip between length-prefix and greedy
    // by editing one attribute).
    let count_attr = node.attribute("count").map(|s| s.to_string());
    let until_eof_raw = node.attribute("until-eof").map(|s| s.to_string());
    let until_eof = match until_eof_raw.as_deref() {
        None => false,
        Some("true") => true,
        Some("false") => false,
        Some(other) => {
            return Err(located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:repeat id='{id}'>"),
                    attr: "sce:until-eof".into(),
                    value: other.to_string(),
                    expected: "\"true\" or \"false\"".into(),
                },
            ));
        }
    };
    let count_ref = match (count_attr, until_eof) {
        (Some(target), false) => CountRef::LengthField(target),
        (None, true) => CountRef::UntilEof,
        (Some(_), true) | (None, false) => {
            return Err(located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:repeat id='{id}'>"),
                    attr: "sce:count / sce:until-eof".into(),
                    value: "<both or neither>".into(),
                    expected: "exactly one of sce:count=\"<sibling_field_id>\" \
                               or sce:until-eof=\"true\""
                        .into(),
                },
            ));
        }
    };

    let max_count = node.attribute("max-count").and_then(parse_int);

    // RFC §5.B B5-μ — present-if on `<sce:repeat>` (Wire RFC Phase B
    // X1). Parses the optional `sce:present-if` attribute identically
    // to `<sce:field>` (`<carrier>.<flag>` Local, `parent.<flag>`
    // Parent, optional `!` negation). Cross-field co-gating with the
    // count source field runs in `validate_codec_repeat_present_if_
    // co_gating` after the field list assembles.
    let present_if = match sce_attr(node, "present-if") {
        None => None,
        Some(raw) => Some(parse_present_if_predicate(&raw, node, doc_name, &id)?),
    };

    Ok(CodecField {
        id,
        // Wire-shape sentinel — the host-language type is derived
        // from `repeat_body_alias` at codegen time. Bytes is the
        // closest primitive (the encoded form is a byte sequence
        // representing concatenated imported-codec encodings).
        sce_type: SceType::Bytes,
        byte_offset,
        bit_offset: None,
        bit_size: BitSize::Repeat { count_ref },
        endian: None,
        max_size: None,
        length_field: None,
        flags: Vec::new(),
        present_if,
        repeat_body_alias: Some(body_alias),
        max_count,
        tlv_chain_body_alias: None,
        dma_burst_align: None,
        length_arith: None,
    })
}

/// RFC §5.B B3 TLV chain primitive — parse `<sce:tlv-chain id="..."
/// sce:type="<imported_alias>" sce:byte="N" max-depth="N"
/// [on-overflow="reject|truncate"]/>`.
///
/// MCU-class. `max-depth` is required at parse time (RFC line 488 "MUST
/// be specified for MCU targets" + B3 v1 only emits to MCU backends);
/// missing → typed `codec/tlv-chain-depth-unspecified` so the author
/// gets a precise repair hint. `on-overflow` defaults to `reject` (RFC
/// line 488 lists `reject` first; matches the safer interpretation
/// — silent drop requires explicit opt-in).
///
/// Wire shape mirrors `<sce:repeat sce:until-eof="true">` — the chain
/// iteratively decodes entries off the cursor; each entry's id+len+body
/// shape is enforced inside the imported entry codec (RFC line 488
/// "Bounded extension list, each has id+len+body"), not by the chain
/// itself.
fn parse_codec_tlv_chain_from_node(
    node: &roxmltree::Node,
    label: DocumentLabel<'_>,
) -> Result<CodecField, Located<ForgeError>> {
    let doc_name = label.diagnostic_label;
    let id = node
        .attribute("id")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: "<sce:tlv-chain>".into(),
                    attr: "id".into(),
                },
            )
        })?
        .to_string();

    // `type` carries the imported entry codec alias verbatim (mirrors
    // `<sce:repeat type=...>`); resolution against `<sce:import>`
    // aliases happens downstream at codegen time.
    let body_alias = node
        .attribute("type")
        .ok_or_else(|| {
            located(
                node,
                doc_name,
                ValidationError::MissingAttribute {
                    element: format!("<sce:tlv-chain id='{id}'>"),
                    attr: "type".into(),
                },
            )
        })?
        .to_string();

    let byte_offset_str = sce_attr(node, "byte").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::MissingAttribute {
                element: format!("<sce:tlv-chain id='{id}'>"),
                attr: "sce:byte".into(),
            },
        )
    })?;
    let byte_offset = parse_int(&byte_offset_str).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::NumericParse {
                element: format!("<sce:tlv-chain id='{id}'>"),
                attr: "sce:byte".into(),
                value: byte_offset_str.clone(),
                detail: "expected integer".into(),
            },
        )
    })?;

    // `max-depth` is mandatory (RFC line 488). Missing → typed
    // tlv-chain-depth-unspecified so the author sees the MCU-class
    // contract explicitly rather than the generic missing-attribute
    // diagnostic.
    let max_depth_raw = node.attribute("max-depth").ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::CodecTlvChainDepthUnspecified {
                codec: label.identifier.to_string(),
                field: id.clone(),
            },
        )
    })?;
    let max_depth = parse_int(max_depth_raw).ok_or_else(|| {
        located(
            node,
            doc_name,
            ValidationError::NumericParse {
                element: format!("<sce:tlv-chain id='{id}'>"),
                attr: "max-depth".into(),
                value: max_depth_raw.to_string(),
                detail: "expected positive integer".into(),
            },
        )
    })?;
    if max_depth == 0 {
        return Err(located(
            node,
            doc_name,
            ValidationError::InvalidAttribute {
                element: format!("<sce:tlv-chain id='{id}'>"),
                attr: "max-depth".into(),
                value: max_depth_raw.to_string(),
                expected: "positive integer (max-depth=\"0\" decodes nothing)".into(),
            },
        ));
    }

    let on_overflow = match node.attribute("on-overflow") {
        None | Some("reject") => TlvOverflowPolicy::Reject,
        Some("truncate") => TlvOverflowPolicy::Truncate,
        Some("diagnostic-event") => {
            return Err(located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:tlv-chain id='{id}'>"),
                    attr: "on-overflow".into(),
                    value: "diagnostic-event".into(),
                    expected:
                        "\"reject\" or \"truncate\" — \"diagnostic-event\" defers to a later \
                         B-stage when §5.A diagnostic-event runtime infrastructure ships"
                            .into(),
                },
            ));
        }
        Some(other) => {
            return Err(located(
                node,
                doc_name,
                ValidationError::InvalidAttribute {
                    element: format!("<sce:tlv-chain id='{id}'>"),
                    attr: "on-overflow".into(),
                    value: other.to_string(),
                    expected: "\"reject\" or \"truncate\"".into(),
                },
            ));
        }
    };

    Ok(CodecField {
        id,
        sce_type: SceType::Bytes,
        byte_offset,
        bit_offset: None,
        bit_size: BitSize::TlvChain {
            max_depth,
            on_overflow,
        },
        endian: None,
        max_size: None,
        length_field: None,
        flags: Vec::new(),
        present_if: None,
        repeat_body_alias: None,
        max_count: None,
        tlv_chain_body_alias: Some(body_alias),
        dma_burst_align: None,
        length_arith: None,
    })
}

/// RFC §5.B B2 repeat cross-field validation. Walks the field list
/// in source order; every `BitSize::Repeat { LengthField(id) }` must
/// RFC §5.B B5-κ Surface L — `sce:length-field` cross-field validation
/// for the dotted-path form `<carrier>.<flag>`. The plain bare-id form
/// is left untyped (codegen-time lookup at the use site, mirroring the
/// pre-B5-κ behavior). Mirrors the B1-δ present-if validator on three
/// axes: forward-reference, carrier-shape, flag existence — plus a
/// fourth gate specific to length-field semantics: the referenced flag
/// must be MULTI-BIT (`width > 1`), since a single-bit flag's value is
/// only ever 0 or 1 — that's the present-if grammar's purpose, not
/// length-source.
///
/// All four failure modes fold into `validation/invalid-attribute`
/// because the repair is attribute-text-level (pick a different
/// carrier, declare the missing flag, widen the flag, or reorder the
/// declarations). No new diagnostic.
fn validate_codec_length_field_refs(
    fields: &[CodecField],
    label: DocumentLabel<'_>,
    datamodel: &roxmltree::Node,
) -> Result<(), Located<ForgeError>> {
    use std::collections::BTreeMap;
    let mut by_id_so_far: BTreeMap<&str, &CodecField> = BTreeMap::new();
    for field in fields {
        if let Some(raw) = field.length_field.as_deref() {
            if let Some((carrier_id, flag_name)) = raw.split_once('.') {
                let carrier_id = carrier_id.trim();
                let flag_name = flag_name.trim();
                let invalid = |expected: String| {
                    located(
                        datamodel,
                        label.diagnostic_label,
                        ValidationError::InvalidAttribute {
                            element: format!(
                                "field '{}' in codec '{}'",
                                field.id, label.identifier
                            ),
                            attr: "sce:length-field".into(),
                            value: raw.to_string(),
                            expected,
                        },
                    )
                };
                if carrier_id.is_empty() || flag_name.is_empty() {
                    return Err(invalid(
                        "dotted-path 'sce:length-field=\"<carrier>.<flag>\"' \
                         requires non-empty carrier and flag identifiers"
                            .into(),
                    ));
                }
                let carrier = by_id_so_far.get(carrier_id).ok_or_else(|| {
                    invalid(format!(
                        "carrier '{carrier_id}' must be a flags-bearing field \
                         declared earlier in the same codec (forward references \
                         are rejected so the streaming decoder reads the carrier \
                         before reaching the length-ref payload)"
                    ))
                })?;
                if !carrier.is_flags_carrier() {
                    return Err(invalid(format!(
                        "dotted-path LHS must reference a flags-bearing carrier \
                         (declared via <sce:flags>); '{carrier_id}' is a plain field"
                    )));
                }
                let flag = carrier.flags.iter().find(|f| f.name == flag_name).ok_or_else(|| {
                    let known: Vec<&str> = carrier.flags.iter().map(|f| f.name.as_str()).collect();
                    invalid(format!(
                        "flag name must be declared on carrier '{carrier_id}': \
                         known flags = [{}]",
                        known.join(", ")
                    ))
                })?;
                if flag.width <= 1 {
                    return Err(invalid(format!(
                        "flag '{carrier_id}.{flag_name}' has width={} but \
                         length-source semantics require multi-bit \
                         (width > 1); single-bit flags are the domain of \
                         sce:present-if, not sce:length-field",
                        flag.width
                    )));
                }
            }
        }
        by_id_so_far.insert(field.id.as_str(), field);
    }
    Ok(())
}

/// reference a previously-declared sibling field whose `sce_type` is
/// integer (so the decoded value is a valid element count).
///
/// Forward / unknown count target → typed
/// `codec/repeat-count-refs-later-field` (the repair is structural —
/// reorder the count to come before the repeat). Non-integer count
/// target reuses the generic `validation/invalid-attribute` (the
/// repair is "fix the attribute text" — pick a different field or
/// retype the existing one).
///
/// `CountRef::UntilEof` needs no validation here: there is no field
/// reference, the loop terminates on cursor exhaustion alone.
fn validate_codec_repeat_count_refs(
    fields: &[CodecField],
    label: DocumentLabel<'_>,
    datamodel: &roxmltree::Node,
) -> Result<(), Located<ForgeError>> {
    use std::collections::BTreeMap;
    let mut by_id_so_far: BTreeMap<&str, &CodecField> = BTreeMap::new();
    for field in fields {
        if let BitSize::Repeat { count_ref: CountRef::LengthField(target) } = &field.bit_size {
            match by_id_so_far.get(target.as_str()) {
                None => {
                    return Err(located(
                        datamodel,
                        label.diagnostic_label,
                        ValidationError::CodecRepeatCountRefsLaterField {
                            codec: label.identifier.to_string(),
                            field: field.id.clone(),
                            refers_to: target.clone(),
                        },
                    ));
                }
                Some(carrier) => {
                    let is_int = carrier.sce_type.is_unsigned()
                        || carrier.sce_type.is_signed();
                    if !is_int {
                        return Err(located(
                            datamodel,
                            label.diagnostic_label,
                            ValidationError::InvalidAttribute {
                                element: format!(
                                    "<sce:repeat id='{}'> in codec '{}'",
                                    field.id, label.identifier
                                ),
                                attr: "sce:count".into(),
                                value: target.clone(),
                                expected: format!(
                                    "count target must be an integer field; \
                                     '{}' is {:?}",
                                    target, carrier.sce_type
                                ),
                            },
                        ));
                    }
                }
            }
        }
        by_id_so_far.insert(field.id.as_str(), field);
    }
    Ok(())
}

/// RFC §5.B B5-μ — repeat-with-present-if co-gating validator (Wire
/// RFC Phase B X1). When `<sce:repeat sce:count="X" sce:present-if="P"/>`
/// is gated, the count source field `X` MUST carry the IDENTICAL
/// predicate `P`. Wire semantics: when the gate fires off the count
/// bytes are absent, so the streaming decoder cannot read `X` to
/// drive the repeat loop unless both share the same predicate. The
/// True-arm of the repeat then reads `X.unwrap()` (or per-language
/// equivalent) safely — the validator is the proof.
///
/// `CountRef::UntilEof` skips this check (no count target). Non-gated
/// repeat skips (back-compat — B2-α trunk shape). Predicate identity
/// compares all four fields of `PresentIfPredicate` (scope, field_id,
/// flag_name, negate); any drift folds into `validation/invalid-
/// attribute` with a precise repair hint naming both fields.
fn validate_codec_repeat_present_if_co_gating(
    fields: &[CodecField],
    label: DocumentLabel<'_>,
    datamodel: &roxmltree::Node,
) -> Result<(), Located<ForgeError>> {
    for field in fields {
        let Some(repeat_pred) = &field.present_if else {
            continue;
        };
        let BitSize::Repeat { count_ref: CountRef::LengthField(target) } = &field.bit_size else {
            continue;
        };
        let count_field = fields
            .iter()
            .find(|f| f.id.as_str() == target.as_str())
            .expect("validate_codec_repeat_count_refs ensured target exists");
        let count_pred = match &count_field.present_if {
            Some(p) => p,
            None => {
                return Err(located(
                    datamodel,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: format!(
                            "<sce:repeat id='{}'> in codec '{}'",
                            field.id, label.identifier
                        ),
                        attr: "sce:present-if".into(),
                        value: format_present_if_predicate_for_diag(repeat_pred),
                        expected: format!(
                            "co-gating contract: count source field '{}' must \
                             carry the IDENTICAL sce:present-if predicate so \
                             the count byte(s) and repeat block share the same \
                             wire-presence gate; '{}' currently has no \
                             present-if",
                            target, target
                        ),
                    },
                ));
            }
        };
        if count_pred != repeat_pred {
            return Err(located(
                datamodel,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: format!(
                        "<sce:repeat id='{}'> in codec '{}'",
                        field.id, label.identifier
                    ),
                    attr: "sce:present-if".into(),
                    value: format!(
                        "repeat='{}' vs count='{}'",
                        format_present_if_predicate_for_diag(repeat_pred),
                        format_present_if_predicate_for_diag(count_pred)
                    ),
                    expected: format!(
                        "co-gating contract: count source field '{}' MUST carry \
                         the IDENTICAL sce:present-if predicate as the repeat — \
                         scope, field_id, flag_name, and negate must match",
                        target
                    ),
                },
            ));
        }
    }
    Ok(())
}

/// Render a `PresentIfPredicate` back to its source-text form for
/// diagnostic messages. Mirrors `parse_present_if_predicate` inverse
/// so the repair hint reads as the author wrote it.
fn format_present_if_predicate_for_diag(p: &PresentIfPredicate) -> String {
    let prefix = if p.negate { "!" } else { "" };
    match p.scope {
        PresentIfScope::Local => format!("{prefix}{}.{}", p.field_id, p.flag_name),
        PresentIfScope::Parent => format!("{prefix}parent.{}", p.flag_name),
    }
}

/// RFC §5.B B3 DMA alignment cross-field validation. For every field
/// carrying `sce:dma-burst-align="N"`:
///
/// 1. The field's authored `sce:byte` MUST be divisible by N (the
///    primitive guarantees a wire-level alignment, so the offset
///    itself has to be aligned — author, not codegen, owns the byte
///    layout).
/// 2. Every preceding field MUST be Fixed bit-size (RFC line 558-583
///    "fixed-offset positions only — no VLE-following alignment").
///    Variable-length predecessors (Vle / LengthRef / Tail / Repeat /
///    TlvChain) make the wire offset of the aligned field runtime-
///    dependent, so static padding cannot honor the constraint.
///
/// Both failure modes fold into `codec/dma-alignment-unsatisfiable`
/// (the repair is structural — either reorder fields, lower the
/// alignment requirement, or change the variable predecessor to a
/// fixed-width carrier). Power-of-2 / parse validation on N already
/// happened in `parse_codec_field_from_node`.
fn validate_codec_dma_alignment(
    fields: &[CodecField],
    label: DocumentLabel<'_>,
    datamodel: &roxmltree::Node,
) -> Result<(), Located<ForgeError>> {
    for (idx, field) in fields.iter().enumerate() {
        let Some(burst_align) = field.dma_burst_align else {
            continue;
        };
        // Gate 1: byte offset divisible by burst-align.
        if field.byte_offset % burst_align != 0 {
            return Err(located(
                datamodel,
                label.diagnostic_label,
                ValidationError::CodecDmaAlignmentUnsatisfiable {
                    codec: label.identifier.to_string(),
                    field: field.id.clone(),
                    burst_align,
                    reason: format!(
                        "field's authored sce:byte={} is not divisible by burst-align {} \
                         (offset must land on a {}-byte boundary; the closest aligned \
                         offsets are {} and {})",
                        field.byte_offset,
                        burst_align,
                        burst_align,
                        (field.byte_offset / burst_align) * burst_align,
                        ((field.byte_offset / burst_align) + 1) * burst_align,
                    ),
                },
            ));
        }
        // Gate 2: every preceding field must be Fixed.
        for prev in &fields[..idx] {
            if !matches!(prev.bit_size, BitSize::Fixed { .. }) {
                let kind = match &prev.bit_size {
                    BitSize::Tail => "tail",
                    BitSize::LengthRef => "length-ref",
                    BitSize::Vle { .. } => "vle",
                    BitSize::Repeat { .. } => "repeat",
                    BitSize::TlvChain { .. } => "tlv-chain",
                    BitSize::Fixed { .. } => unreachable!("matches! guard"),
                };
                return Err(located(
                    datamodel,
                    label.diagnostic_label,
                    ValidationError::CodecDmaAlignmentUnsatisfiable {
                        codec: label.identifier.to_string(),
                        field: field.id.clone(),
                        burst_align,
                        reason: format!(
                            "preceding field '{}' has bit-size '{}' (variable-length); static padding \
                             cannot honor sce:dma-burst-align when any prior field's wire size depends \
                             on runtime values (RFC §5.B \"fixed-offset positions only — no VLE-\
                             following alignment\")",
                            prev.id, kind,
                        ),
                    },
                ));
            }
        }
    }
    Ok(())
}

// ── RFC §5.B B5-θ codec test-vector parsing ────────────────────
//
// `<sce:test-vector hex="cafe">
//    <sce:decoded field="sn" value="1"/>
//    <sce:decoded field="payload" hex="ca fe"/>
//  </sce:test-vector>`
//
// Trunk shape: each row binds one wire byte sequence to a flat
// list of `<sce:decoded>` field assignments. Field name must
// resolve to a `CodecField` declared in the same codec; the
// value form (`value=` / `hex=` / `string=`) must match that
// field's `SceType` (parser rejects mismatches via
// `validation/invalid-attribute`). Variant / TLV-chain / parent-
// flags codecs reject downstream at the per-language sidecar
// emitter so the parser surface stays uniform.

fn parse_codec_test_vectors(
    root: &roxmltree::Node,
    fields: &[CodecField],
    label: DocumentLabel<'_>,
) -> Result<Vec<CodecTestVector>, Located<ForgeError>> {
    let mut vectors = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE)
            || child.tag_name().name() != "test-vector"
        {
            continue;
        }
        vectors.push(parse_one_codec_test_vector(&child, fields, label)?);
    }
    Ok(vectors)
}

fn parse_one_codec_test_vector(
    node: &roxmltree::Node,
    fields: &[CodecField],
    label: DocumentLabel<'_>,
) -> Result<CodecTestVector, Located<ForgeError>> {
    let hex_attr = node.attribute("hex").ok_or_else(|| {
        located(
            node,
            label.diagnostic_label,
            ValidationError::MissingAttribute {
                element: "sce:test-vector".into(),
                attr: "hex".into(),
            },
        )
    })?;
    let hex = decode_hex(&strip_hex_whitespace(hex_attr)).map_err(|reason| {
        located(
            node,
            label.diagnostic_label,
            ValidationError::InvalidAttribute {
                element: "sce:test-vector".into(),
                attr: "hex".into(),
                value: hex_attr.to_string(),
                expected: reason,
            },
        )
    })?;

    let mut decoded_fields = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        match child.tag_name().name() {
            "decoded" => {
                decoded_fields.push(parse_one_decoded_field(&child, fields, label)?);
            }
            other => {
                return Err(located(
                    &child,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "sce:test-vector".into(),
                        attr: "<child element>".into(),
                        value: format!("sce:{other}"),
                        expected: "B5-θ trunk only accepts <sce:decoded field=\"...\" \
                                   value|hex|string=\"...\"/> children; \
                                   <sce:decoded-variant>/<sce:decoded-chain>/<sce:decoded-entry> \
                                   defer to B5-θ closures"
                            .into(),
                    },
                ));
            }
        }
    }

    Ok(CodecTestVector {
        hex,
        decoded: DecodedValue::Plain { fields: decoded_fields },
        source_line: node.document().text_pos_at(node.range().start).row as usize,
    })
}

fn parse_one_decoded_field(
    node: &roxmltree::Node,
    fields: &[CodecField],
    label: DocumentLabel<'_>,
) -> Result<DecodedField, Located<ForgeError>> {
    let name = node.attribute("field").ok_or_else(|| {
        located(
            node,
            label.diagnostic_label,
            ValidationError::MissingAttribute {
                element: "sce:decoded".into(),
                attr: "field".into(),
            },
        )
    })?;
    let codec_field = fields.iter().find(|f| f.id == name).ok_or_else(|| {
        located(
            node,
            label.diagnostic_label,
            ValidationError::InvalidAttribute {
                element: "sce:decoded".into(),
                attr: "field".into(),
                value: name.to_string(),
                expected: format!(
                    "field name must resolve to a <sce:field>/<sce:flags> declared in this \
                     codec; available: {}",
                    fields
                        .iter()
                        .map(|f| f.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            },
        )
    })?;

    let value_attr = node.attribute("value");
    let hex_attr = node.attribute("hex");
    let string_attr = node.attribute("string");
    let n_set =
        usize::from(value_attr.is_some()) + usize::from(hex_attr.is_some()) + usize::from(string_attr.is_some());
    if n_set != 1 {
        return Err(located(
            node,
            label.diagnostic_label,
            ValidationError::InvalidAttribute {
                element: "sce:decoded".into(),
                attr: "value|hex|string".into(),
                value: format!("{n_set} of value/hex/string attributes set"),
                expected: "exactly one of value=, hex=, or string= must be set per <sce:decoded> row"
                    .into(),
            },
        ));
    }

    let typed = match &codec_field.sce_type {
        SceType::Bytes => {
            let raw = hex_attr.ok_or_else(|| located(
                node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "sce:decoded".into(),
                    attr: "value-form".into(),
                    value: "value= or string=".into(),
                    expected: format!(
                        "field '{}' has SceType::Bytes — must use hex=\"...\" form",
                        codec_field.id
                    ),
                },
            ))?;
            let bytes = decode_hex(&strip_hex_whitespace(raw)).map_err(|reason| {
                located(
                    node,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "sce:decoded".into(),
                        attr: "hex".into(),
                        value: raw.to_string(),
                        expected: reason,
                    },
                )
            })?;
            DecodedFieldValue::Bytes(bytes)
        }
        SceType::String => {
            let s = string_attr.ok_or_else(|| located(
                node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "sce:decoded".into(),
                    attr: "value-form".into(),
                    value: "value= or hex=".into(),
                    expected: format!(
                        "field '{}' has SceType::String — must use string=\"...\" form",
                        codec_field.id
                    ),
                },
            ))?;
            DecodedFieldValue::String(s.to_string())
        }
        SceType::Bool => {
            let v = value_attr.ok_or_else(|| located(
                node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "sce:decoded".into(),
                    attr: "value-form".into(),
                    value: "hex= or string=".into(),
                    expected: format!(
                        "field '{}' has SceType::Bool — must use value=\"true|false\"",
                        codec_field.id
                    ),
                },
            ))?;
            match v.trim() {
                "true" => DecodedFieldValue::Bool(true),
                "false" => DecodedFieldValue::Bool(false),
                _ => {
                    return Err(located(
                        node,
                        label.diagnostic_label,
                        ValidationError::InvalidAttribute {
                            element: "sce:decoded".into(),
                            attr: "value".into(),
                            value: v.to_string(),
                            expected: "boolean literal 'true' or 'false'".into(),
                        },
                    ));
                }
            }
        }
        ty if ty.is_unsigned() || ty.is_signed() => {
            let v = value_attr.ok_or_else(|| located(
                node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "sce:decoded".into(),
                    attr: "value-form".into(),
                    value: "hex= or string=".into(),
                    expected: format!(
                        "field '{}' has integer SceType — must use value=\"...\" form",
                        codec_field.id
                    ),
                },
            ))?;
            parse_decoded_int_literal(v, ty).map_err(|reason| {
                located(
                    node,
                    label.diagnostic_label,
                    ValidationError::InvalidAttribute {
                        element: "sce:decoded".into(),
                        attr: "value".into(),
                        value: v.to_string(),
                        expected: reason,
                    },
                )
            })?
        }
        other => {
            return Err(located(
                node,
                label.diagnostic_label,
                ValidationError::InvalidAttribute {
                    element: "sce:decoded".into(),
                    attr: "field".into(),
                    value: name.to_string(),
                    expected: format!(
                        "field '{}' has SceType {other:?} which is not yet supported in B5-θ \
                         test vectors (Bool/integer/Bytes/String only); float closures defer to \
                         the first float-bearing codec consumer",
                        codec_field.id
                    ),
                },
            ));
        }
    };

    Ok(DecodedField {
        name: name.to_string(),
        value: typed,
    })
}

/// Strip ASCII whitespace from a hex string so authors can use
/// space-separated nibbles for readability (`"01 02 03"` ≡ `"010203"`).
fn strip_hex_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_ascii_whitespace()).collect()
}

fn parse_decoded_int_literal(s: &str, return_type: &SceType) -> Result<DecodedFieldValue, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("value attribute is empty".into());
    }
    let (negative, digits) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest.trim_start())
    } else {
        (false, trimmed)
    };
    let magnitude = if let Some(rest) = digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).map_err(|e| format!("invalid hex literal after '0x': {e}"))?
    } else if let Some(rest) = digits.strip_prefix("0b").or_else(|| digits.strip_prefix("0B")) {
        u64::from_str_radix(rest, 2).map_err(|e| format!("invalid binary literal after '0b': {e}"))?
    } else {
        digits
            .parse::<u64>()
            .map_err(|e| format!("invalid decimal integer literal: {e}"))?
    };
    if negative {
        if !return_type.is_signed() {
            return Err(format!(
                "negative value not allowed for unsigned field type {return_type:?}"
            ));
        }
        let signed = i64::try_from(magnitude)
            .map(|n| -n)
            .map_err(|_| format!("value '-{magnitude}' overflows i64"))?;
        Ok(DecodedFieldValue::Int(signed))
    } else if return_type.is_signed() {
        let signed = i64::try_from(magnitude)
            .map_err(|_| format!("value '{magnitude}' overflows i64"))?;
        Ok(DecodedFieldValue::Int(signed))
    } else {
        Ok(DecodedFieldValue::Uint(magnitude))
    }
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

    let test_vectors = parse_test_vectors(root, &signature, label.diagnostic_label)?;

    Ok(AlgorithmModel {
        name: label.identifier.to_string(),
        signature,
        consts,
        body,
        test_vectors,
    })
}

// ── RFC §5.B test-vector parsing (B2-test-vector-prep) ──────────
//
// `<sce:test-vector hex="313233343536373839" value="0x29B1"/>` —
// inline reference oracle. v1 covers algorithm kind only with scalar
// return; multi-field codec test vectors defer to B5. Both attributes
// are required; hex must be even-length hex-only; value must be a
// numeric or boolean literal compatible with the algorithm's declared
// return type (mismatches reuse `validation/invalid-attribute` —
// repair stays attribute-text-level).

fn parse_test_vectors(
    root: &roxmltree::Node,
    signature: &AlgorithmSignature,
    diagnostic_label: &str,
) -> Result<Vec<TestVector>, Located<ForgeError>> {
    let mut vectors = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE)
            || child.tag_name().name() != "test-vector"
        {
            continue;
        }
        vectors.push(parse_one_test_vector(&child, signature, diagnostic_label)?);
    }
    Ok(vectors)
}

fn parse_one_test_vector(
    node: &roxmltree::Node,
    signature: &AlgorithmSignature,
    diagnostic_label: &str,
) -> Result<TestVector, Located<ForgeError>> {
    let hex_attr = node.attribute("hex").ok_or_else(|| {
        located(
            node,
            diagnostic_label,
            ValidationError::MissingAttribute {
                element: "sce:test-vector".into(),
                attr: "hex".into(),
            },
        )
    })?;
    let value_attr = node.attribute("value").ok_or_else(|| {
        located(
            node,
            diagnostic_label,
            ValidationError::MissingAttribute {
                element: "sce:test-vector".into(),
                attr: "value".into(),
            },
        )
    })?;

    let hex = decode_hex(hex_attr).map_err(|reason| {
        located(
            node,
            diagnostic_label,
            ValidationError::InvalidAttribute {
                element: "sce:test-vector".into(),
                attr: "hex".into(),
                value: hex_attr.to_string(),
                expected: reason,
            },
        )
    })?;

    let return_type = signature.return_type.as_ref().ok_or_else(|| {
        located(
            node,
            diagnostic_label,
            ValidationError::InvalidAttribute {
                element: "sce:test-vector".into(),
                attr: "value".into(),
                value: value_attr.to_string(),
                expected: "<sce:test-vector> requires a non-void return type on the algorithm signature; declare <sce:return type=\"...\"/> before adding test vectors".into(),
            },
        )
    })?;

    let value = parse_test_vector_value(value_attr, return_type).map_err(|reason| {
        located(
            node,
            diagnostic_label,
            ValidationError::InvalidAttribute {
                element: "sce:test-vector".into(),
                attr: "value".into(),
                value: value_attr.to_string(),
                expected: reason,
            },
        )
    })?;

    Ok(TestVector {
        hex,
        value,
        source_line: node.document().text_pos_at(node.range().start).row as usize,
    })
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.is_empty() {
        return Ok(Vec::new());
    }
    if s.len() % 2 != 0 {
        return Err(format!(
            "hex string must have an even number of digits (got {} characters)",
            s.len()
        ));
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let bs = s.as_bytes();
    for i in (0..bs.len()).step_by(2) {
        let hi = hex_nibble(bs[i])?;
        let lo = hex_nibble(bs[i + 1])?;
        bytes.push((hi << 4) | lo);
    }
    Ok(bytes)
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(10 + (b - b'a')),
        b'A'..=b'F' => Ok(10 + (b - b'A')),
        _ => Err(format!(
            "invalid hex character '{}' (allowed: 0-9, a-f, A-F)",
            b as char
        )),
    }
}

fn parse_test_vector_value(s: &str, return_type: &SceType) -> Result<TestVectorValue, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("value attribute is empty".into());
    }
    if matches!(return_type, SceType::Bool) {
        return match trimmed {
            "true" => Ok(TestVectorValue::Bool(true)),
            "false" => Ok(TestVectorValue::Bool(false)),
            _ => Err(format!(
                "expected boolean literal 'true' or 'false' for bool return type (got '{trimmed}')"
            )),
        };
    }
    let is_integer = return_type.is_signed() || return_type.is_unsigned();
    if !is_integer {
        return Err(format!(
            "<sce:test-vector value> only supports bool/integer scalar return types in v1; got '{return_type:?}' — multi-field codec or float-result test vectors defer to B5"
        ));
    }

    let (negative, digits) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest.trim_start())
    } else {
        (false, trimmed)
    };

    let magnitude = if let Some(rest) = digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).map_err(|e| format!(
            "invalid hex literal after '0x': {e}"
        ))?
    } else if let Some(rest) = digits.strip_prefix("0b").or_else(|| digits.strip_prefix("0B")) {
        u64::from_str_radix(rest, 2).map_err(|e| format!(
            "invalid binary literal after '0b': {e}"
        ))?
    } else {
        digits.parse::<u64>().map_err(|e| format!(
            "invalid decimal integer literal: {e}"
        ))?
    };

    if negative {
        if !return_type.is_signed() {
            return Err(format!(
                "negative value not allowed for unsigned return type {return_type:?}"
            ));
        }
        let signed = i64::try_from(magnitude)
            .map(|n| -n)
            .map_err(|_| format!("value '-{magnitude}' overflows i64"))?;
        Ok(TestVectorValue::Int(signed))
    } else if return_type.is_signed() {
        let signed = i64::try_from(magnitude)
            .map_err(|_| format!("value '{magnitude}' overflows i64"))?;
        Ok(TestVectorValue::Int(signed))
    } else {
        Ok(TestVectorValue::Uint(magnitude))
    }
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
