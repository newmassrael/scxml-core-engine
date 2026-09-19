//! NL→IR Mapping Roadmap Item 5 — strict-mode + reporting reader
//! for `<sce:unresolved>` placeholders.
//!
//! ⚠ It has no walk of its own, and the absence is the repair. It used
//! to keep one, and that walk stopped short of a transition's own
//! actions, of everything nested inside `<if>` / `<foreach>`, and of the
//! `<initial>` and `<history>` default actions — so `--strict-unresolved`
//! passed placeholders the parser had stored and the manifest
//! classification reported as `unresolved`. It now reads
//! [`crate::requirements_report::walk_nodes`], the traversal the report,
//! the classification and the transition table already share, so a
//! marker one of them can see is a marker all of them can.
//!
//! Two consumer surfaces share this reader:
//!
//! - `--strict-unresolved` (CLI flag on `generate`): if any IR node
//!   carries an [`UnresolvedMarker`], the build fails with
//!   `ValidationError::UnresolvedPlaceholder` keyed at the first
//!   offending node — CI gates cannot merge an unresolved IR.
//!
//! - `sce-codegen unresolved <file>`: emit one NDJSON record per
//!   marker in document order so IDE / linter / dashboard consumers
//!   can surface the open decisions without parsing SCXML
//!   themselves.

use std::io::{self, Write};

use serde::Serialize;

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::model::SCXMLModel;
use crate::provenance::{MarkerKind, SpecProvenance, UnresolvedMarker};
use crate::requirements_report::{walk_nodes, ActionSite, NodeSubject};

/// The first unresolved placeholder in a model, with everything the
/// rejection needs to describe the node that owns it.
///
/// A named struct rather than a tuple because the third member is the
/// one a reader would otherwise have to guess at: it is the node's
/// spec anchors, not the marker's.
pub struct Unresolved<'a> {
    /// Author-facing element label, e.g. `<state id="armed">`.
    pub element: String,
    pub marker: &'a UnresolvedMarker,
    /// `sce:provenance` of the node that owns the marker, verbatim.
    pub provenance: &'a [SpecProvenance],
}

/// Walk `model` in document order and return the first
/// `UnresolvedMarker`, the author-facing element label of the node
/// that owns it, and that node's `sce:provenance` anchors. `None`
/// means the model is clean — `--strict-unresolved` lets the build
/// proceed.
///
/// The anchors ride along because this walker is the one place that
/// knows WHICH node the rejection is about: an unresolved placeholder
/// is a question, and the spec paragraph the author anchored the node
/// at is where its answer lives. Returning the label without them
/// would hand a triager an SCXML line and stop.
/// ⚠ `MarkerKind::Assumed` is deliberately NOT returned here. An
/// assumption is on the record but does not block — the distinction is
/// argued at [`MarkerKind`]. Filtering at this one function is what
/// keeps the reports (which list both) and the refusal (which lists one)
/// reading the same markers.
pub fn first_unresolved(model: &SCXMLModel) -> Option<Unresolved<'_>> {
    walk_nodes(model).into_iter().find_map(|node| {
        let marker = node
            .subject
            .unresolved()
            .iter()
            .find(|m| m.kind == MarkerKind::Unresolved)?;
        Some(Unresolved {
            element: element_label(&node.subject),
            marker,
            provenance: node.record.spec_provenance,
        })
    })
}

/// The element a rejection names, in the vocabulary the author wrote.
///
/// The labels for a state, a transition, an `<onentry>` / `<onexit>`
/// action and an invoke are the ones this check printed before it moved
/// onto the shared walk, byte for byte. The three action sites that walk
/// newly reaches name their container the same way.
fn element_label(subject: &NodeSubject<'_>) -> String {
    match subject {
        NodeSubject::State(state) => format!("<state id=\"{}\">", state.id),
        NodeSubject::Transition { state, index, .. } => {
            format!("<transition #{index} in <state id=\"{}\">>", state.id)
        }
        NodeSubject::Action {
            state,
            action,
            site,
        } => {
            let container = match site {
                ActionSite::Entry => "<onentry>".to_string(),
                ActionSite::Exit => "<onexit>".to_string(),
                ActionSite::Transition { index } => format!("<transition #{index}>"),
                ActionSite::Initial => "<initial>".to_string(),
                ActionSite::HistoryDefault { history } => format!("<history id=\"{history}\">"),
            };
            format!(
                "<{} in {container} of <state id=\"{}\">>",
                action.action_type, state.id
            )
        }
        NodeSubject::Invoke { state, index, base } => format!(
            "<invoke #{index} (id=\"{}\") in <state id=\"{}\">>",
            base.invoke_id, state.id
        ),
        // No owning state to name: §scxml-5.8 puts this one at the
        // document, so the label says the document rather than
        // borrowing a state it does not sit in.
        NodeSubject::GlobalScript { index, .. } => {
            format!("<script #{index} at the top level of <scxml>>")
        }
    }
}

/// Strict-mode gate. Returns `Err` iff the model carries at least
/// one [`UnresolvedMarker`]. The marker's `id` + `reason` are
/// surfaced through [`ValidationError::UnresolvedPlaceholder`] —
/// downstream NDJSON consumers route on the wire `code` =
/// `validation/unresolved-placeholder`.
pub fn check_strict_unresolved(model: &SCXMLModel) -> Result<(), Located<ForgeError>> {
    match first_unresolved(model) {
        None => Ok(()),
        Some(found) => {
            let location = found.marker.location.clone().unwrap_or(SourceLocation {
                file: String::new(),
                line: None,
                col: None,
            });
            Err(Located::new(
                ValidationError::UnresolvedPlaceholder {
                    element: found.element,
                    id: found.marker.id.clone(),
                    reason: found.marker.reason.clone(),
                }
                .into(),
                location.file,
                location.line,
                location.col,
            )
            // The node's spec anchors ride onto the wire's
            // `spec_provenance`, so a CI gate that refuses the build
            // hands the reader the document to go read, not only the
            // line to go look at.
            .with_spec_provenance(found.provenance.to_vec()))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct UnresolvedRecord<'a> {
    node_path: String,
    node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_type: Option<&'a str>,
    /// `unresolved` or `assumed`. A consumer that renders a work queue
    /// needs it: the first is a question to send the spec's author, the
    /// second is a decision already taken that the author should confirm.
    /// Without it both arrive as one undifferentiated list.
    kind: MarkerKind,
    id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    candidates: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<&'a SourceLocation>,
}

/// Emit one NDJSON record per `<sce:unresolved>` marker. Stable
/// document order. Empty output when the model is clean.
///
/// `?Sized` for the same reason as
/// [`emit_requirements_ndjson`](crate::requirements_report::emit_requirements_ndjson):
/// the CLI streams this into an erased stdout sink.
pub fn emit_unresolved_ndjson<W: Write + ?Sized>(
    model: &SCXMLModel,
    writer: &mut W,
) -> io::Result<()> {
    for node in walk_nodes(model) {
        let action_type = match &node.subject {
            NodeSubject::Action { action, .. } => Some(action.action_type.as_str()),
            _ => None,
        };
        for marker in node.subject.unresolved() {
            write_marker(
                writer,
                &node.record.node_path,
                node.record.node_type,
                action_type,
                marker,
            )?;
        }
    }
    Ok(())
}

fn write_marker<W: Write + ?Sized>(
    writer: &mut W,
    node_path: &str,
    node_type: &'static str,
    action_type: Option<&str>,
    marker: &UnresolvedMarker,
) -> io::Result<()> {
    let record = UnresolvedRecord {
        node_path: node_path.to_string(),
        node_type,
        action_type,
        kind: marker.kind,
        id: marker.id.as_str(),
        reason: marker.reason.as_deref(),
        candidates: marker.candidates.iter().map(|s| s.as_str()).collect(),
        location: marker.location.as_ref(),
    };
    let line = serde_json::to_string(&record)
        .expect("UnresolvedRecord serialises; all fields owned or borrowed primitives");
    writeln!(writer, "{line}")
}

// ── The same two surfaces, for a forge-kind document ────────────────
//
// ⚠ WHY THIS EXISTS AT ALL. Measured 2026-09-18: a `sce:kind="transform"`
// document carrying `sce:unresolved` on a `<data>` node generated with
// `--strict-unresolved` and exit 0, and `sce-codegen unresolved` refused
// the file outright ("transform kind cannot be processed by the SCXML
// pipeline"). The marker was accepted by the XML and understood by
// nothing — the worst shape a safety mechanism can take, because the
// author who writes "I do not know this value" is told nothing and the
// build ships the guess. Every surface above served statecharts only.
//
// ⚠⚠ WHY THE XML AND NOT THE MODEL. The statechart side stores markers
// in `SCXMLModel` and walks it. The forge side has seventeen kinds, each
// with its own model and its own field containers, so the equivalent
// walk would be a seventeen-arm match that a new kind silently falls out
// of — the exact staleness this tree has been bitten by elsewhere. The
// markers live in the XML in one shape for every kind, so reading them
// there cannot go stale, and a kind added tomorrow is covered the day it
// parses.
//
// ⚠⚠⚠ THE RESIDUE, STATED RATHER THAN HIDDEN: because the marker is not
// lifted into the forge model, it does not appear in `--emit-ast` and a
// consumer reading the AST cannot see it. That is a real gap for the
// round-trip law a pseudocode surface would need, and it is a larger
// change (seventeen models) than the defect above justified on its own.
// It is recorded here rather than left for someone to discover.

/// Every `<sce:unresolved>` marker anywhere in a forge document, in
/// document order, paired with the element that owns it.
fn forge_markers(
    content: &str,
    source_name: &str,
) -> Result<Vec<(String, UnresolvedMarker)>, Located<ForgeError>> {
    // The same error the forge parser raises for malformed XML. In
    // practice unreachable from `generate` — the caller parsed this very
    // content a moment ago — but returning `Ok(vec![])` here would report
    // "no unresolved markers" for a document nobody could read, which is
    // the shape this whole module exists to refuse.
    let doc = roxmltree::Document::parse(content).map_err(|e| {
        Located::new(
            crate::forge::error::XmlError::Parse(e.to_string()).into(),
            source_name.to_string(),
            None,
            None,
        )
    })?;
    let mut out = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        for marker in crate::parser::collect_sce_unresolved(&node, source_name) {
            // `<data id="x">` reads better in a refusal than `data`,
            // and the id is what the author named the thing.
            let label = match node.attribute("id") {
                Some(id) => format!("<{} id=\"{}\">", node.tag_name().name(), id),
                None => format!("<{}>", node.tag_name().name()),
            };
            out.push((label, marker));
        }
    }
    Ok(out)
}

/// `--strict-unresolved` for a forge document: refuse the build when any
/// marker is present, keyed at the first one in document order.
pub fn check_strict_unresolved_forge(
    content: &str,
    source_name: &str,
) -> Result<(), Located<ForgeError>> {
    let markers = forge_markers(content, source_name)?;
    // Same filter as `first_unresolved`, same reason — see [`MarkerKind`].
    match markers
        .into_iter()
        .find(|(_, m)| m.kind == MarkerKind::Unresolved)
    {
        None => Ok(()),
        Some((element, marker)) => {
            let location = marker.location.clone().unwrap_or(SourceLocation {
                file: source_name.to_string(),
                line: None,
                col: None,
            });
            Err(Located::new(
                ValidationError::UnresolvedPlaceholder {
                    element,
                    id: marker.id.clone(),
                    reason: marker.reason.clone(),
                }
                .into(),
                location.file,
                location.line,
                location.col,
            ))
        }
    }
}

/// `sce-codegen unresolved` for a forge document — one NDJSON record per
/// marker, the same record shape the statechart path emits so a consumer
/// does not branch on the kind it was handed.
pub fn emit_unresolved_ndjson_forge<W: Write + ?Sized>(
    content: &str,
    source_name: &str,
    writer: &mut W,
) -> Result<(), Located<ForgeError>> {
    for (element, marker) in forge_markers(content, source_name)? {
        // `node_type` is "forge" for every kind rather than the kind's own
        // name: a consumer routes on `code` and reads `node_path`, and a
        // per-kind spelling here would be a second place the seventeen
        // kinds have to stay listed.
        let _ = write_marker(writer, &element, "forge", None, &marker);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(xml: &str) -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(xml, "unresolved_test")
            .expect("parse")
    }

    #[test]
    fn attribute_form_marker_collected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="armed" datamodel="null">
                <state id="armed"
                       sce:unresolved="tbd_threshold"
                       sce:unresolved-reason="awaiting calibration"
                       sce:unresolved-candidates="42 50 65"/>
            </scxml>"#,
        );
        let armed = &model.states["armed"];
        assert_eq!(armed.unresolved.len(), 1);
        let marker = &armed.unresolved[0];
        assert_eq!(marker.id, "tbd_threshold");
        assert_eq!(marker.reason.as_deref(), Some("awaiting calibration"));
        assert_eq!(marker.candidates, vec!["42", "50", "65"]);
    }

    #[test]
    fn element_form_marker_collected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="armed" datamodel="null">
                <state id="armed">
                    <sce:unresolved id="tbd_target" reason="route TBD" candidates="left right"/>
                </state>
            </scxml>"#,
        );
        let armed = &model.states["armed"];
        assert_eq!(armed.unresolved.len(), 1);
        let marker = &armed.unresolved[0];
        assert_eq!(marker.id, "tbd_target");
        assert_eq!(marker.reason.as_deref(), Some("route TBD"));
        assert_eq!(marker.candidates, vec!["left", "right"]);
    }

    #[test]
    fn strict_check_fails_on_first_marker() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="armed" datamodel="null">
                <state id="armed" sce:unresolved="tbd_id" sce:unresolved-reason="why"/>
            </scxml>"#,
        );
        let err = check_strict_unresolved(&model).expect_err("strict mode must fail");
        let rendered = format!("{err:?}");
        assert!(
            rendered.contains("UnresolvedPlaceholder"),
            "expected ValidationError::UnresolvedPlaceholder, got: {rendered}"
        );
        assert!(rendered.contains("tbd_id"), "missing id in: {rendered}");
    }

    #[test]
    fn strict_check_passes_when_clean() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          version="1.0" initial="armed" datamodel="null">
                <state id="armed"/>
            </scxml>"#,
        );
        assert!(check_strict_unresolved(&model).is_ok());
    }

    #[test]
    fn ndjson_empty_when_clean() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          version="1.0" initial="armed" datamodel="null">
                <state id="armed"/>
            </scxml>"#,
        );
        let mut buf = Vec::new();
        emit_unresolved_ndjson(&model, &mut buf).unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn ndjson_emits_one_record_per_marker() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="armed" datamodel="null">
                <state id="armed" sce:unresolved="m1">
                    <onentry>
                        <raise event="ev" sce:unresolved="m2"/>
                    </onentry>
                    <transition event="go" target="armed" sce:unresolved="m3"/>
                </state>
            </scxml>"#,
        );
        let mut buf = Vec::new();
        emit_unresolved_ndjson(&model, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 records, got: {out}");
        assert!(lines[0].contains("\"id\":\"m1\""));
        assert!(lines[1].contains("\"id\":\"m3\""));
        assert!(lines[2].contains("\"id\":\"m2\""));
    }
}
