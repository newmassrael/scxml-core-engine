//! NL→IR Mapping Roadmap Item 5 — strict-mode + reporting walker
//! for `<sce:unresolved>` placeholders.
//!
//! Two consumer surfaces share this walker:
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
use crate::model::{Invoke, SCXMLModel, State};
use crate::provenance::UnresolvedMarker;

/// Walk `model` in document order and return the first
/// `UnresolvedMarker` paired with the author-facing element label
/// of the node that owns it. `None` means the model is clean —
/// `--strict-unresolved` lets the build proceed.
pub fn first_unresolved(model: &SCXMLModel) -> Option<(String, &UnresolvedMarker)> {
    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        if let Some(m) = state.unresolved.first() {
            return Some((format!("<state id=\"{}\">", state.id), m));
        }
        for (i, transition) in state.transitions.iter().enumerate() {
            if let Some(m) = transition.unresolved.first() {
                return Some((
                    format!("<transition #{i} in <state id=\"{}\">>", state.id),
                    m,
                ));
            }
        }
        for block in state.on_entry_blocks.iter() {
            for action in block.iter() {
                if let Some(m) = action.unresolved.first() {
                    return Some((
                        format!(
                            "<{} in <onentry> of <state id=\"{}\">>",
                            action.action_type, state.id
                        ),
                        m,
                    ));
                }
            }
        }
        for block in state.on_exit_blocks.iter() {
            for action in block.iter() {
                if let Some(m) = action.unresolved.first() {
                    return Some((
                        format!(
                            "<{} in <onexit> of <state id=\"{}\">>",
                            action.action_type, state.id
                        ),
                        m,
                    ));
                }
            }
        }
        for (i, invoke) in state.invokes.iter().enumerate() {
            let base = match invoke {
                Invoke::Scxml(info) => &info.common.base,
                Invoke::Hybrid(info) => &info.common.base,
                Invoke::MeshRpc(info) => &info.base,
                Invoke::Unsupported(info) => &info.base,
            };
            if let Some(m) = base.unresolved.first() {
                return Some((
                    format!(
                        "<invoke #{i} (id=\"{}\") in <state id=\"{}\">>",
                        base.invoke_id, state.id
                    ),
                    m,
                ));
            }
        }
    }
    None
}

/// Strict-mode gate. Returns `Err` iff the model carries at least
/// one [`UnresolvedMarker`]. The marker's `id` + `reason` are
/// surfaced through [`ValidationError::UnresolvedPlaceholder`] —
/// downstream NDJSON consumers route on the wire `code` =
/// `validation/unresolved-placeholder`.
pub fn check_strict_unresolved(model: &SCXMLModel) -> Result<(), Located<ForgeError>> {
    match first_unresolved(model) {
        None => Ok(()),
        Some((element, marker)) => {
            let location = marker.location.clone().unwrap_or(SourceLocation {
                file: String::new(),
                line: None,
                col: None,
            });
            Err(Located {
                error: ValidationError::UnresolvedPlaceholder {
                    element,
                    id: marker.id.clone(),
                    reason: marker.reason.clone(),
                }
                .into(),
                location,
            })
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct UnresolvedRecord<'a> {
    node_path: String,
    node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_type: Option<&'a str>,
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
pub fn emit_unresolved_ndjson<W: Write>(model: &SCXMLModel, writer: &mut W) -> io::Result<()> {
    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        let node_path_state = format!("states.{}", state.id);
        for marker in &state.unresolved {
            write_marker(writer, &node_path_state, "state", None, marker)?;
        }
        for (i, transition) in state.transitions.iter().enumerate() {
            let path = format!("states.{}.transitions[{i}]", state.id);
            for marker in &transition.unresolved {
                write_marker(writer, &path, "transition", None, marker)?;
            }
        }
        for (i, block) in state.on_entry_blocks.iter().enumerate() {
            for (j, action) in block.iter().enumerate() {
                let path = format!("states.{}.on_entry_blocks[{i}][{j}]", state.id);
                for marker in &action.unresolved {
                    write_marker(
                        writer,
                        &path,
                        "action",
                        Some(action.action_type.as_str()),
                        marker,
                    )?;
                }
            }
        }
        for (i, block) in state.on_exit_blocks.iter().enumerate() {
            for (j, action) in block.iter().enumerate() {
                let path = format!("states.{}.on_exit_blocks[{i}][{j}]", state.id);
                for marker in &action.unresolved {
                    write_marker(
                        writer,
                        &path,
                        "action",
                        Some(action.action_type.as_str()),
                        marker,
                    )?;
                }
            }
        }
        for (i, invoke) in state.invokes.iter().enumerate() {
            let base = match invoke {
                Invoke::Scxml(info) => &info.common.base,
                Invoke::Hybrid(info) => &info.common.base,
                Invoke::MeshRpc(info) => &info.base,
                Invoke::Unsupported(info) => &info.base,
            };
            let path = format!("states.{}.invokes[{i}]", state.id);
            for marker in &base.unresolved {
                write_marker(writer, &path, "invoke", None, marker)?;
            }
        }
    }
    Ok(())
}

fn write_marker<W: Write>(
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
        id: marker.id.as_str(),
        reason: marker.reason.as_deref(),
        candidates: marker.candidates.iter().map(|s| s.as_str()).collect(),
        location: marker.location.as_ref(),
    };
    let line = serde_json::to_string(&record)
        .expect("UnresolvedRecord serialises; all fields owned or borrowed primitives");
    writeln!(writer, "{line}")
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
