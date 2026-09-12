//! NL→IR Mapping Roadmap Items 1 + 7 — emit a requirement-coverage
//! NDJSON for a parsed [`crate::model::SCXMLModel`].
//!
//! One JSON record per IR node carrying a non-empty `sce:req` or
//! `sce:provenance` annotation, in document order, on stdout. Nodes
//! carrying neither are skipped — a document that annotates nothing
//! produces no output at all, and a document that annotates only with
//! `sce:req` produces byte-identical output to the pre-Item-7 report.
//!
//! `sce:provenance` widens the emit condition rather than riding only
//! on `sce:req`-bearing records, and that is the point of the report
//! rather than a convenience: what a consumer compares against the
//! revisions actually in force is the set of `(doc_id, rev)` pairs the
//! document claims to depend on, and a pair that reached the IR but
//! not the report would shrink that set without saying so. A node can
//! be anchored at a spec section without naming a requirement ID —
//! the two annotations are orthogonal axes.
//!
//! Driven by the `sce-codegen requirements <file.scxml>`
//! subcommand. Consumers (req-coverage reporters, IDE linters,
//! compliance auditors) parse stdout line-by-line and dispatch on
//! `node_type`.

use std::io::{self, Write};

use serde::Serialize;

use crate::forge::error::SourceLocation;
use crate::model::SCXMLModel;
use crate::provenance::{RequirementId, SpecProvenance};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RequirementRecord<'a> {
    /// Hierarchical path identifying the IR node — e.g.
    /// `"states.armed"`, `"states.armed.transitions[0]"`,
    /// `"states.armed.on_entry_blocks[0][1]"`,
    /// `"states.armed.invokes[0]"`. Stable across re-parses of the
    /// same document.
    pub(crate) node_path: String,
    /// Lowercase short tag for routing — `state` / `transition` /
    /// `action` / `invoke`. Action records additionally carry the
    /// SCXML element name (`raise`/`send`/...) on `action_type`.
    pub(crate) node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_type: Option<&'a str>,
    /// Verbatim requirement ids in document order. Emitted even when
    /// empty: it is the record's defining field, and a consumer
    /// reading `record.requirement_ids` must not find it undefined on
    /// a node annotated only with `sce:provenance`.
    pub(crate) requirement_ids: Vec<&'a str>,
    /// `sce:provenance` spec-document anchors in document order,
    /// verbatim from the IR — SCE never reads the documents they name.
    ///
    /// Omitted entirely when the node carries none, which is what
    /// keeps a document without `sce:provenance` byte-identical to the
    /// pre-Item-7 report.
    #[serde(skip_serializing_if = "<[SpecProvenance]>::is_empty")]
    pub(crate) spec_provenance: &'a [SpecProvenance],
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<&'a SourceLocation>,
}

/// Emit one NDJSON record per IR node carrying `sce:req` IDs or
/// `sce:provenance` anchors.
/// Records are written in stable document order: states sorted by
/// `document_order`, with transitions → on_entry_blocks →
/// on_exit_blocks → invokes inside each state.
///
/// `?Sized` so a caller holding an erased sink (`&mut dyn Write`) can
/// stream into it. The CLI does: every one of its stdout writers goes
/// through one failure-handling helper, and that helper cannot be
/// generic over the writer and still be one function.
pub fn emit_requirements_ndjson<W: Write + ?Sized>(
    model: &SCXMLModel,
    writer: &mut W,
) -> io::Result<()> {
    for node in annotated_nodes(model) {
        write_record(writer, &node.record)?;
    }
    Ok(())
}

/// One annotated IR node, as the walk found it.
///
/// [`RequirementRecord`] is the *wire* projection and carries only what
/// the NDJSON consumer contract promises. This carries that plus what a
/// caller inside the crate needs and the wire does not offer — today,
/// whether the node also holds an `<sce:unresolved>` marker, which
/// `requirement_manifest` needs to tell an honest "I did not know" from
/// a claim of implementation.
///
/// Kept a separate type rather than widening the record, because the
/// record's shape is a promise to consumers parsing stdout line by
/// line: a field added for an in-crate caller would change every line
/// of every report to serve something no consumer asked for.
pub(crate) struct AnnotatedNode<'a> {
    pub(crate) record: RequirementRecord<'a>,
    /// Whether this node carries at least one `<sce:unresolved>`.
    pub(crate) unresolved: bool,
}

/// Every annotated node in stable document order — states sorted by
/// `document_order`, and inside each state transitions →
/// on_entry_blocks → on_exit_blocks → invokes.
///
/// THE traversal. `emit_requirements_ndjson` is a writer over it and
/// `requirement_manifest` is a set comparison over it, so the two
/// cannot disagree about which nodes are annotated or what their
/// `node_path` is — a second walk written to answer the manifest
/// question would be free to drift from the one the report prints, and
/// the whole value of the classification is that it names nodes a
/// reader can find in that report.
pub(crate) fn annotated_nodes(model: &SCXMLModel) -> Vec<AnnotatedNode<'_>> {
    let mut out = Vec::new();
    let mut states: Vec<&crate::model::State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        if is_annotated(&state.req, &state.provenance) {
            out.push(AnnotatedNode {
                record: RequirementRecord {
                    node_path: format!("states.{}", state.id),
                    node_type: "state",
                    action_type: None,
                    requirement_ids: refs_of(&state.req),
                    spec_provenance: &state.provenance,
                    location: state.source_location.as_ref(),
                },
                unresolved: !state.unresolved.is_empty(),
            });
        }
        for (i, transition) in state.transitions.iter().enumerate() {
            if is_annotated(&transition.req, &transition.provenance) {
                out.push(AnnotatedNode {
                    record: RequirementRecord {
                        node_path: format!("states.{}.transitions[{i}]", state.id),
                        node_type: "transition",
                        action_type: None,
                        requirement_ids: refs_of(&transition.req),
                        spec_provenance: &transition.provenance,
                        location: transition.source_location.as_ref(),
                    },
                    unresolved: !transition.unresolved.is_empty(),
                });
            }
        }
        for (i, block) in state.on_entry_blocks.iter().enumerate() {
            for (j, action) in block.iter().enumerate() {
                if is_annotated(&action.req, &action.provenance) {
                    out.push(AnnotatedNode {
                        record: RequirementRecord {
                            node_path: format!("states.{}.on_entry_blocks[{i}][{j}]", state.id),
                            node_type: "action",
                            action_type: Some(action.action_type.as_str()),
                            requirement_ids: refs_of(&action.req),
                            spec_provenance: &action.provenance,
                            location: action.source_location.as_ref(),
                        },
                        unresolved: !action.unresolved.is_empty(),
                    });
                }
            }
        }
        for (i, block) in state.on_exit_blocks.iter().enumerate() {
            for (j, action) in block.iter().enumerate() {
                if is_annotated(&action.req, &action.provenance) {
                    out.push(AnnotatedNode {
                        record: RequirementRecord {
                            node_path: format!("states.{}.on_exit_blocks[{i}][{j}]", state.id),
                            node_type: "action",
                            action_type: Some(action.action_type.as_str()),
                            requirement_ids: refs_of(&action.req),
                            spec_provenance: &action.provenance,
                            location: action.source_location.as_ref(),
                        },
                        unresolved: !action.unresolved.is_empty(),
                    });
                }
            }
        }
        for (i, invoke) in state.invokes.iter().enumerate() {
            let base = match invoke {
                crate::model::Invoke::Scxml(info) => &info.common.base,
                crate::model::Invoke::Hybrid(info) => &info.common.base,
                crate::model::Invoke::MeshRpc(info) => &info.base,
                crate::model::Invoke::Unsupported(info) => &info.base,
            };
            if is_annotated(&base.req, &base.provenance) {
                out.push(AnnotatedNode {
                    record: RequirementRecord {
                        node_path: format!("states.{}.invokes[{i}]", state.id),
                        node_type: "invoke",
                        action_type: None,
                        requirement_ids: refs_of(&base.req),
                        spec_provenance: &base.provenance,
                        location: None,
                    },
                    unresolved: !base.unresolved.is_empty(),
                });
            }
        }
    }
    out
}

/// Whether a node has anything to report. One predicate rather than
/// the condition spelled at each of the five node kinds, so a third
/// annotation family widens the report in one place — and so the five
/// cannot disagree about what "annotated" means, which is the failure
/// the `sce:provenance` half of this file already had once: the field
/// was on the model and no record carried it.
fn is_annotated(req: &[RequirementId], provenance: &[SpecProvenance]) -> bool {
    !req.is_empty() || !provenance.is_empty()
}

fn refs_of(ids: &[RequirementId]) -> Vec<&str> {
    ids.iter().map(|r| r.0.as_str()).collect()
}

fn write_record<W: Write + ?Sized>(
    writer: &mut W,
    record: &RequirementRecord<'_>,
) -> io::Result<()> {
    let line = serde_json::to_string(record)
        .expect("RequirementRecord serialises; all fields are owned or borrowed primitives");
    writeln!(writer, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    #[test]
    fn emits_record_per_annotated_node_in_document_order() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed" sce:req="REQ_STATE">
                <onentry sce:req="REQ_ONENTRY">
                    <raise event="ev1"/>
                </onentry>
                <transition event="go" target="firing" sce:req="REQ_TRANS"/>
                <invoke type="scxml" src="child.scxml" sce:req="REQ_INVOKE"/>
            </state>
            <state id="firing"/>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "report_test")
            .expect("parse");
        let mut buf = Vec::new();
        emit_requirements_ndjson(&model, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(
            lines.len(),
            4,
            "expected 4 NDJSON records (state, action via onentry, transition, invoke); got: {out}"
        );
        assert!(lines[0].contains("\"node_type\":\"state\""));
        assert!(lines[0].contains("\"REQ_STATE\""));
        assert!(lines[1].contains("\"node_type\":\"transition\""));
        assert!(lines[1].contains("\"REQ_TRANS\""));
        assert!(lines[2].contains("\"node_type\":\"action\""));
        assert!(lines[2].contains("\"REQ_ONENTRY\""));
        assert!(lines[3].contains("\"node_type\":\"invoke\""));
        assert!(lines[3].contains("\"REQ_INVOKE\""));
    }

    #[test]
    fn spec_provenance_rides_the_record_of_the_node_that_carries_it() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed" sce:req="REQ_STATE"
                   sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
              <sce:provenance doc-id="OEM-TIMING-REQ" rev="B" section="7.1"/>
              <transition event="go" target="firing" sce:req="REQ_TRANS"/>
            </state>
            <state id="firing"/>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "report_provenance")
            .expect("parse");
        let mut buf = Vec::new();
        emit_requirements_ndjson(&model, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "expected state + transition; got: {out}");
        // Both anchors, in document order, with the compact form's
        // optional parts expanded and its absent ones omitted.
        assert!(
            lines[0].contains(
                r#""spec_provenance":[{"doc_id":"OEM-DIAG-SPEC","rev":"D","section":"3.4.2","page":112},{"doc_id":"OEM-TIMING-REQ","rev":"B","section":"7.1"}]"#
            ),
            "state record must carry both anchors verbatim: {}",
            lines[0],
        );
        // The transition is annotated only with `sce:req`, so the
        // field is absent rather than an empty array — that omission
        // is what keeps unannotated documents byte-identical.
        assert!(
            !lines[1].contains("spec_provenance"),
            "a node with no anchor must omit the field entirely: {}",
            lines[1],
        );
    }

    #[test]
    fn a_node_anchored_without_a_requirement_id_still_reports() {
        // The `(doc_id, rev)` set is what a consumer compares against
        // the revisions in force, so an anchor that reached the IR has
        // to reach the report — whether or not the node also names a
        // requirement. Emitting only `sce:req`-bearing records would
        // drop this pair silently.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed" sce:provenance="OEM-DIAG-SPEC@D">
                <onentry><raise event="ev1" sce:provenance="ISO-14229-1#11.2.1"/></onentry>
            </state>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "report_provenance_only")
            .expect("parse");
        let mut buf = Vec::new();
        emit_requirements_ndjson(&model, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "expected state + action; got: {out}");
        assert!(lines[0].contains(r#""requirement_ids":[]"#));
        assert!(lines[0].contains(r#""spec_provenance":[{"doc_id":"OEM-DIAG-SPEC","rev":"D"}]"#));
        assert!(lines[1].contains(r#""node_type":"action""#));
        assert!(
            lines[1].contains(r#""spec_provenance":[{"doc_id":"ISO-14229-1","section":"11.2.1"}]"#),
            "the action record must carry its own anchor: {}",
            lines[1],
        );
    }

    #[test]
    fn a_sce_req_only_document_is_byte_identical_to_the_pre_item_7_report() {
        // RFC §6.3 at the report layer: adding the field must not move
        // one byte of a document that declares no anchor. Pinned as a
        // literal rather than as "does not contain spec_provenance",
        // which a reordered or renamed field would still satisfy.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed" sce:req="REQ_STATE"/>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "report_req_only")
            .expect("parse");
        let mut buf = Vec::new();
        emit_requirements_ndjson(&model, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "{\"node_path\":\"states.armed\",\"node_type\":\"state\",\
             \"requirement_ids\":[\"REQ_STATE\"],\
             \"location\":{\"file\":\"report_req_only\",\"line\":4,\"col\":13}}\n",
        );
    }

    #[test]
    fn no_records_when_attribute_absent() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed">
                <transition event="go" target="armed"/>
            </state>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "report_absent")
            .expect("parse");
        let mut buf = Vec::new();
        emit_requirements_ndjson(&model, &mut buf).unwrap();
        assert!(buf.is_empty(), "absent attribute → empty NDJSON output");
    }
}
