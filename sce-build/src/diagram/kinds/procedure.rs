// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A procedure as the statechart figure of its states.
//!
//! A procedure is a flat graph: states, the transitions between them, the
//! values some transitions assign, and the final states it ends in. That is
//! exactly what the statechart figure draws, with its numbered arrows and
//! the table of transitions under it, so this module does not lay a graph
//! out a second time: it states the procedure's graph as the small SCXML
//! document it is — the states, each transition's event, condition and
//! target, its assignments, the final states' result parameters — and
//! hands that to [`crate::diagram::fit::print`].
//!
//! What the figure does not carry is what a statechart has no place for:
//! the `send` each state fires on entry, the helpers, the inputs and
//! internals. Those are in the field table and on the pseudocode page; the
//! figure is the graph of how the procedure moves, which is the part that
//! is a picture.

use crate::diagram::fit::{print, Page, Printed, Refusal};
use crate::forge::model::ProcedureModel;
use crate::forge::page::Lexicon;
use crate::parser::{escape_xml_attribute as attr, SCXMLParser};
use std::fmt::Write as _;

/// The procedure's graph as an SCXML document.
fn scxml_of(m: &ProcedureModel) -> String {
    let mut out = String::new();
    out.push_str(r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0""#);
    if !m.initial.is_empty() {
        let _ = write!(out, r#" initial="{}""#, attr(&m.initial));
    }
    out.push_str(">\n");
    for state in &m.states {
        let tag = if state.is_final { "final" } else { "state" };
        let _ = writeln!(out, r#"  <{tag} id="{}">"#, attr(&state.id));
        for t in &state.transitions {
            let _ = write!(out, "    <transition");
            if let Some(event) = &t.event {
                let _ = write!(out, r#" event="{}""#, attr(event));
            }
            if let Some(cond) = &t.cond {
                let _ = write!(out, r#" cond="{}""#, attr(cond));
            }
            let _ = writeln!(out, r#" target="{}">"#, attr(&t.target));
            for a in &t.assigns {
                let _ = writeln!(
                    out,
                    r#"      <assign location="{}" expr="{}"/>"#,
                    attr(&a.location),
                    attr(&a.expr)
                );
            }
            out.push_str("    </transition>\n");
        }
        if !state.done_params.is_empty() {
            out.push_str("    <donedata>\n");
            for p in &state.done_params {
                let _ = writeln!(
                    out,
                    r#"      <param name="{}" expr="{}"/>"#,
                    attr(&p.name),
                    attr(&p.expr)
                );
            }
            out.push_str("    </donedata>\n");
        }
        let _ = writeln!(out, "  </{tag}>");
    }
    out.push_str("</scxml>\n");
    out
}

/// The figures of `m`: the statechart figure of its states, and its
/// transition table. None for a procedure with no state.
pub fn figures(m: &ProcedureModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Printed>, Refusal> {
    if m.states.is_empty() {
        return Ok(Vec::new());
    }
    let model = SCXMLParser::new()
        .parse_string(&scxml_of(m), &m.name)
        .map_err(|e| Refusal::Unreadable(format!("the procedure's states as a statechart: {e}")))?;
    print(&model, lexicon, page)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::FigureName;
    use crate::forge::model::ForgeDocument;
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn parse(path: &std::path::Path) -> Option<ProcedureModel> {
        let text = std::fs::read_to_string(path).expect("reads");
        match parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
        {
            Ok(Some(parsed)) => match parsed.document {
                ForgeDocument::Procedure(m) => Some(m),
                _ => None,
            },
            _ => None,
        }
    }

    fn example() -> ProcedureModel {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join("procedure.scxml");
        parse(&path).expect("a procedure")
    }

    fn page() -> Page {
        Page::named("a3-landscape", 7.0).expect("a listed page")
    }

    /// The graph handed to the statechart parser says what the model
    /// says: every state, every transition's event, condition and target,
    /// every assignment, every result parameter.
    #[test]
    fn the_scxml_states_the_procedures_graph() {
        let m = example();
        let xml = scxml_of(&m);
        let doc = roxmltree::Document::parse(&xml).expect("well-formed");
        let ids: Vec<&str> = doc
            .descendants()
            .filter(|n| matches!(n.tag_name().name(), "state" | "final"))
            .filter_map(|n| n.attribute("id"))
            .collect();
        let wanted: Vec<&str> = m.states.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, wanted);
        let transitions = doc
            .descendants()
            .filter(|n| n.has_tag_name("transition"))
            .count();
        assert_eq!(
            transitions,
            m.states.iter().map(|s| s.transitions.len()).sum::<usize>()
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_tag_name("assign"))
                .count(),
            m.states
                .iter()
                .flat_map(|s| &s.transitions)
                .map(|t| t.assigns.len())
                .sum::<usize>()
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_tag_name("param"))
                .count(),
            m.states.iter().map(|s| s.done_params.len()).sum::<usize>()
        );
        assert!(doc
            .descendants()
            .any(|n| n.attribute("cond") == Some("retryCount < maxRetries")));
    }

    /// The authenticate procedure is one figure of its states, with every
    /// state in a box and every transition in the table under it.
    #[test]
    fn the_example_is_one_figure_of_its_states_and_a_table_of_transitions() {
        let m = example();
        let printed = figures(&m, &EN, page()).expect("draws");
        assert_eq!(printed.len(), 1);
        assert_eq!(printed[0].laid.name, FigureName::Document);
        let svg = crate::diagram::svg::render(&printed[0]);
        let words: Vec<String> = roxmltree::Document::parse(&svg)
            .expect("well-formed")
            .descendants()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| n.text().map(str::to_string))
            .collect();
        for id in [
            "ping",
            "requestChallenge",
            "answer",
            "retry",
            "done",
            "failed",
        ] {
            // A final state's box carries the page's word for it after
            // its name (`done  final`).
            assert!(
                words
                    .iter()
                    .any(|w| w == id || w.starts_with(&format!("{id}  "))),
                "{id}: {words:?}"
            );
        }
        let rows: usize = printed.iter().map(|p| p.table.len()).sum();
        assert_eq!(
            rows,
            m.states.iter().map(|s| s.transitions.len()).sum::<usize>()
        );
        let korean = figures(&m, &KO, page()).expect("draws");
        assert_ne!(
            crate::diagram::svg::render(&korean[0]),
            svg,
            "in the page's language"
        );
    }

    /// A procedure with no state has no figure.
    #[test]
    fn a_procedure_with_no_state_has_no_figure() {
        let mut m = example();
        m.states.clear();
        assert!(figures(&m, &EN, page()).expect("draws").is_empty());
    }

    /// Every procedure fixture of the tree draws as a figure whose boxes
    /// and table rows account for its states and transitions; none is
    /// refused.
    #[test]
    fn every_procedure_fixture_draws() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/forge/resources");
        let mut seen = 0;
        for entry in std::fs::read_dir(&dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("scxml") {
                continue;
            }
            let Some(m) = parse(&path) else { continue };
            seen += 1;
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let printed = figures(&m, &EN, page()).unwrap_or_else(|e| panic!("{name}: {e}"));
            let rows: usize = printed.iter().map(|p| p.table.len()).sum();
            assert_eq!(
                rows,
                m.states.iter().map(|s| s.transitions.len()).sum::<usize>(),
                "{name}"
            );
        }
        assert!(seen >= 5, "the procedure fixtures parse: {seen}");
    }
}
