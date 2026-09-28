// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The statechart pseudocode page shows what the document says, and its
//! reader recovers it — for four things it once left out.
//!
//! # What was wrong
//!
//! Measured 2026-09-28, on the page a specification owner reviews:
//!
//! * **Containment.** Every state was written at the top level, so the
//!   page could not say that `inner` is inside `parent` — and which state
//!   a transition leaves, and which one `initial` enters, depend on it
//!   (§scxml-3.3). The reader built a flat machine back, so the round
//!   trip `render(parse(render(m))) == render(m)` passed while both
//!   halves dropped the same thing.
//! * **`<history>`.** Not written at all: a transition to `h` reached the
//!   page and `h` did not.
//! * **`sce:provenance` and markers.** Not written on any node.
//! * **`<scxml name>`.** Not written; the head showed the file's name.
//!
//! # Why the round trip cannot hold this, and what does
//!
//! The round trip proves the reader recovers what the renderer WROTE; it
//! is silent about what the renderer left out. So each case here compares
//! the page against the DOCUMENT — the parser's model, which is where the
//! author's nesting lives — and then reads the page back and compares
//! again.

use sce_build::forge::model::ForgeDocument;
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="door_controller" initial="parent" datamodel="null">
  <state id="parent" sce:provenance="DOOR-SPEC@B#4.2:page=12">
    <state id="inner" sce:assumed="A_LIMIT" sce:assumed-reason="the spec gives no limit"
           sce:assumed-candidates="10 15">
      <transition event="go" target="deep"/>
    </state>
    <state id="deep">
      <history id="h" type="deep">
        <transition target="d1">
          <raise event="restored"/>
        </transition>
      </history>
      <state id="d1"/>
    </state>
  </state>
</scxml>"#;

fn model() -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(DOC, "door")
        .unwrap_or_else(|e| panic!("the document parses: {:?}", e.error))
}

fn page_of(m: &SCXMLModel) -> String {
    sce_build::forge::pseudo::render(&ForgeDocument::Statechart(Box::new(m.clone())))
        .expect("the document renders")
}

fn read(page: &str) -> SCXMLModel {
    match sce_build::forge::unpseudo::parse(page).expect("the page reads back") {
        ForgeDocument::Statechart(m) => *m,
        other => panic!("the page read back as {:?}", other.kind()),
    }
}

/// Indentation of the line that opens a state, in levels of two spaces.
/// The head is `state <id> [clauses]:`, so the id is its second word.
fn depth_of_state(page: &str, id: &str) -> usize {
    let line = page
        .lines()
        .find(|l| {
            let mut words = l.split_whitespace();
            matches!(words.next(), Some("state" | "parallel" | "final"))
                && words.next().map(|w| w.trim_end_matches(':')) == Some(id)
        })
        .unwrap_or_else(|| panic!("the page writes no state `{id}`:\n{page}"));
    (line.len() - line.trim_start().len()) / 2
}

#[test]
fn a_state_is_written_inside_the_state_that_contains_it() {
    let m = model();
    let page = page_of(&m);
    let top = depth_of_state(&page, "parent");
    assert_eq!(depth_of_state(&page, "inner"), top + 1, "{page}");
    assert_eq!(depth_of_state(&page, "deep"), top + 1, "{page}");
    assert_eq!(depth_of_state(&page, "d1"), top + 2, "{page}");

    let back = read(&page);
    for (id, parent) in [("inner", "parent"), ("deep", "parent"), ("d1", "deep")] {
        assert_eq!(
            back.states[id].parent.as_deref(),
            Some(parent),
            "`{id}` came back off the page without its parent"
        );
        assert_eq!(
            back.states[id].parent, m.states[id].parent,
            "the page and the document disagree on `{id}`'s parent"
        );
    }
    assert_eq!(back.states["parent"].children, m.states["parent"].children);
}

#[test]
fn a_history_is_written_with_its_type_default_and_actions() {
    let m = model();
    let page = page_of(&m);
    let line = page
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("history h "))
        .unwrap_or_else(|| panic!("the page writes no `<history id=\"h\">`:\n{page}"));
    assert_eq!(line, "history h deep -> d1");

    let back = read(&page);
    let h = &back.history_states["h"];
    let written = &m.history_states["h"];
    assert_eq!(
        (&h.parent, &h.history_type, &h.default_target),
        (
            &written.parent,
            &written.history_type,
            &written.default_target
        )
    );
    assert_eq!(h.default_actions.len(), 1, "the default transition's raise");
    assert_eq!(h.default_actions[0].event, "restored");
}

#[test]
fn provenance_and_a_marker_are_written_and_read_back() {
    let m = model();
    let page = page_of(&m);
    let lines: Vec<&str> = page.lines().map(str::trim).collect();
    assert!(
        lines.contains(&"provenance DOOR-SPEC@B#4.2:page=12"),
        "{page}"
    );
    assert!(lines.contains(&"assumed A_LIMIT"), "{page}");
    assert!(lines.contains(&"reason the spec gives no limit"), "{page}");
    assert!(lines.contains(&"candidates 10 15"), "{page}");

    let back = read(&page);
    assert_eq!(
        back.states["parent"].provenance,
        m.states["parent"].provenance
    );
    let (marker, written) = (
        &back.states["inner"].unresolved,
        &m.states["inner"].unresolved,
    );
    assert_eq!(marker.len(), 1);
    assert_eq!(
        (
            &marker[0].id,
            marker[0].kind,
            &marker[0].reason,
            &marker[0].candidates
        ),
        (
            &written[0].id,
            written[0].kind,
            &written[0].reason,
            &written[0].candidates
        )
    );
}

#[test]
fn the_machine_name_the_author_wrote_is_on_the_head_line() {
    let m = model();
    let page = page_of(&m);
    let head = page.lines().next().expect("a page has a head");
    assert!(head.contains("name: door_controller"), "{head}");
    assert_eq!(read(&page).scxml_name, "door_controller");
}

/// ⚠ An assignment to a variable called `req` is `req = 1` on the page,
/// the same leading word as an annotation. The reader tells them apart
/// by the `=` — this case is what holds that.
#[test]
fn an_assignment_to_a_variable_named_like_an_annotation_stays_an_assignment() {
    let doc = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s" datamodel="ecmascript">
  <datamodel><data id="req" expr="0"/></datamodel>
  <state id="s">
    <transition event="go" target="s">
      <assign location="req" expr="1"/>
    </transition>
  </state>
</scxml>"#;
    let m = SCXMLParser::new()
        .parse_string(doc, "assign_req")
        .unwrap_or_else(|e| panic!("parses: {:?}", e.error));
    let page = page_of(&m);
    let back = read(&page);
    let t = &back.states["s"].transitions[0];
    assert!(t.req.is_empty(), "`req = 1` was read as a requirement");
    assert_eq!(t.actions.len(), 1, "{page}");
    assert_eq!(
        (t.actions[0].location.as_str(), t.actions[0].expr.as_str()),
        ("req", "1")
    );
}
