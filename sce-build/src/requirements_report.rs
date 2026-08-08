//! NL→IR Mapping Roadmap Item 1 — emit a requirement-coverage
//! NDJSON for a parsed [`crate::model::SCXMLModel`].
//!
//! One JSON record per IR node carrying a non-empty `sce:req`
//! attribute, in document order, on stdout. Nodes without
//! `sce:req` are skipped — absence is byte-identical to the
//! pre-Item-1 state of the report.
//!
//! Driven by the `sce-codegen requirements <file.scxml>`
//! subcommand. Consumers (req-coverage reporters, IDE linters,
//! compliance auditors) parse stdout line-by-line and dispatch on
//! `node_type`.

use std::io::{self, Write};

use serde::Serialize;

use crate::forge::error::SourceLocation;
use crate::model::SCXMLModel;
use crate::provenance::RequirementId;

#[derive(Debug, Clone, Serialize)]
struct RequirementRecord<'a> {
    /// Hierarchical path identifying the IR node — e.g.
    /// `"states.armed"`, `"states.armed.transitions[0]"`,
    /// `"states.armed.on_entry_blocks[0][1]"`,
    /// `"states.armed.invokes[0]"`. Stable across re-parses of the
    /// same document.
    node_path: String,
    /// Lowercase short tag for routing — `state` / `transition` /
    /// `action` / `invoke`. Action records additionally carry the
    /// SCXML element name (`raise`/`send`/...) on `action_type`.
    node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_type: Option<&'a str>,
    /// Verbatim requirement ids in document order.
    requirement_ids: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<&'a SourceLocation>,
}

/// Emit one NDJSON record per IR node carrying `sce:req` IDs.
/// Records are written in stable document order: states sorted by
/// `document_order`, with transitions → on_entry_blocks →
/// on_exit_blocks → invokes inside each state.
pub fn emit_requirements_ndjson<W: Write>(model: &SCXMLModel, writer: &mut W) -> io::Result<()> {
    let mut states: Vec<&crate::model::State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        if !state.req.is_empty() {
            let record = RequirementRecord {
                node_path: format!("states.{}", state.id),
                node_type: "state",
                action_type: None,
                requirement_ids: refs_of(&state.req),
                location: state.source_location.as_ref(),
            };
            write_record(writer, &record)?;
        }
        for (i, transition) in state.transitions.iter().enumerate() {
            if !transition.req.is_empty() {
                let record = RequirementRecord {
                    node_path: format!("states.{}.transitions[{i}]", state.id),
                    node_type: "transition",
                    action_type: None,
                    requirement_ids: refs_of(&transition.req),
                    location: transition.source_location.as_ref(),
                };
                write_record(writer, &record)?;
            }
        }
        for (i, block) in state.on_entry_blocks.iter().enumerate() {
            for (j, action) in block.iter().enumerate() {
                if !action.req.is_empty() {
                    let record = RequirementRecord {
                        node_path: format!("states.{}.on_entry_blocks[{i}][{j}]", state.id),
                        node_type: "action",
                        action_type: Some(action.action_type.as_str()),
                        requirement_ids: refs_of(&action.req),
                        location: action.source_location.as_ref(),
                    };
                    write_record(writer, &record)?;
                }
            }
        }
        for (i, block) in state.on_exit_blocks.iter().enumerate() {
            for (j, action) in block.iter().enumerate() {
                if !action.req.is_empty() {
                    let record = RequirementRecord {
                        node_path: format!("states.{}.on_exit_blocks[{i}][{j}]", state.id),
                        node_type: "action",
                        action_type: Some(action.action_type.as_str()),
                        requirement_ids: refs_of(&action.req),
                        location: action.source_location.as_ref(),
                    };
                    write_record(writer, &record)?;
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
            if !base.req.is_empty() {
                let record = RequirementRecord {
                    node_path: format!("states.{}.invokes[{i}]", state.id),
                    node_type: "invoke",
                    action_type: None,
                    requirement_ids: refs_of(&base.req),
                    location: None,
                };
                write_record(writer, &record)?;
            }
        }
    }
    Ok(())
}

fn refs_of(ids: &[RequirementId]) -> Vec<&str> {
    ids.iter().map(|r| r.0.as_str()).collect()
}

fn write_record<W: Write>(writer: &mut W, record: &RequirementRecord<'_>) -> io::Result<()> {
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
