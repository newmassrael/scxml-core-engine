//! NL→IR closure ledger row G2 — what the diagram is handed.
//!
//! The visualizer cannot be held to this by comparing its output: a
//! rendered diagram is not bytes a test can diff, which is exactly why
//! the Requirement-closure RFC puts the visualizer AFTER the acceptance
//! report rather than before it. So the thing pinned here is the INPUT —
//! the overlay the browser receives — and the property pinned is the one
//! a second walk in the browser would get wrong.
//!
//! # The property
//!
//! A requirement's evidence is not the elements carrying its id. It is
//! the dependency closure of those elements, and over the real pair 39 of
//! 72 (requirement, dependency) pairs lie on nodes carrying no id at all.
//! A browser picking elements by `sce:req` would therefore draw a
//! different answer from the table the machine measures, silently.
//!
//! So: the overlay must reach nodes that carry no id, and it must reach
//! exactly the ones the acceptance report reaches.
//!
//! ⚠ Every assertion here carries a floor or a control, because each has
//! a way of passing vacuously — an empty fragment satisfies "contains no
//! wrong node", and an overlay that put EVERY node in every fragment
//! satisfies "contains the arming send".

use sce_build::acceptance_report::fragment;
use sce_build::annotation_overlay::overlay;
use sce_build::parser::SCXMLParser;

/// A timer armed by a delayed `<send>` and disarmed by a `<cancel>`,
/// neither of which carries the requirement the transition claims. The
/// shape the RFC names as the one a by-id walk loses.
const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="null" name="g2_probe">
  <state id="idle">
    <onentry>
      <send event="tick" delay="3s" id="armTimer"/>
    </onentry>
    <transition event="tick" target="done" sce:req="REQ-A"/>
  </state>
  <state id="done">
    <onentry>
      <cancel sendid="armTimer"/>
    </onentry>
  </state>
  <state id="unrelated">
    <onentry>
      <log label="nothing to do with REQ-A"/>
    </onentry>
  </state>
</scxml>"#;

fn parse() -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(DOC, "g2_probe")
        .expect("the probe document parses")
}

#[test]
fn the_fragment_reaches_nodes_that_carry_no_id() {
    let model = parse();
    let ov = overlay(&model);

    let claims: std::collections::BTreeMap<&str, &Vec<String>> = ov
        .nodes
        .iter()
        .map(|n| (n.node_path.as_str(), &n.requirements))
        .collect();

    let frag = ov
        .fragments
        .iter()
        .find(|f| f.requirement == "REQ-A")
        .expect("REQ-A is claimed by the document");

    let supporting: Vec<&str> = frag
        .dependencies
        .iter()
        .filter(|d| {
            claims
                .get(d.node_path.as_str())
                .map(|r| !r.iter().any(|id| id == "REQ-A"))
                .unwrap_or(false)
        })
        .map(|d| d.node_path.as_str())
        .collect();

    // The floor. "Reaches a node carrying no id" is satisfied by one
    // accident; the arming send AND the cancel are both required, because
    // losing either is the defect this row exists for.
    assert!(
        supporting.len() >= 2,
        "REQ-A's fragment must reach the arming send and the cancel, neither of which \
         carries the id; got {supporting:?} out of {:?}",
        frag.dependencies
    );

    let relations: std::collections::BTreeSet<&str> =
        frag.dependencies.iter().map(|d| d.relation).collect();
    assert!(
        relations.contains("arming-send"),
        "the timer that arms the cited transition is missing: {relations:?}"
    );
    assert!(
        relations.contains("cancel-of-arming-send"),
        "the cancel of that timer is missing: {relations:?}"
    );
}

#[test]
fn the_control_an_unrelated_element_is_not_in_the_fragment() {
    let model = parse();
    let ov = overlay(&model);
    let frag = ov
        .fragments
        .iter()
        .find(|f| f.requirement == "REQ-A")
        .expect("REQ-A is claimed");

    // ⭐ Without this, an overlay that put every node in every fragment
    // would satisfy the test above. `unrelated` shares no event, no
    // target and no timer with REQ-A.
    let reaches_unrelated: Vec<&str> = frag
        .dependencies
        .iter()
        .map(|d| d.node_path.as_str())
        .filter(|p| p.starts_with("states.unrelated"))
        .collect();
    assert!(
        reaches_unrelated.is_empty(),
        "the fragment swept in an element REQ-A does not depend on: {reaches_unrelated:?}"
    );
}

#[test]
fn the_overlay_carries_the_unclaimed_elements_it_is_asked_to_mark() {
    let model = parse();
    let ov = overlay(&model);

    // The reviewer's question is which element NOTHING claims, so those
    // rows must survive into the overlay rather than being filtered out.
    let unclaimed: Vec<&str> = ov.unclaimed().map(|n| n.node_path.as_str()).collect();
    assert!(
        unclaimed.contains(&"states.unrelated"),
        "a state no requirement claims must appear as unclaimed: {unclaimed:?}"
    );
    // Control: the claiming element must NOT be reported unclaimed, or
    // the field says nothing.
    assert!(
        !unclaimed.contains(&"states.idle.transitions[0]"),
        "the transition claiming REQ-A was reported unclaimed"
    );
}

#[test]
fn the_overlay_is_the_reports_closure_and_not_a_second_one() {
    let model = parse();
    let ov = overlay(&model);

    for frag in &ov.fragments {
        let from_report: std::collections::BTreeSet<String> = fragment(&model, &frag.requirement)
            .into_iter()
            .map(|d| d.node_path)
            .collect();
        let from_overlay: std::collections::BTreeSet<String> = frag
            .dependencies
            .iter()
            .map(|d| d.node_path.clone())
            .collect();

        // Not a tautology even though today's overlay calls `fragment`:
        // it is what goes red the moment somebody inserts a filter, a
        // cache or a second derivation between the two — which is the
        // only way this row can quietly reopen.
        assert_eq!(
            from_overlay, from_report,
            "the overlay's closure for {} diverged from the acceptance report's",
            frag.requirement
        );
        assert!(
            !from_report.is_empty(),
            "empty closure for {} would make the comparison vacuous",
            frag.requirement
        );
    }
    assert!(
        !ov.fragments.is_empty(),
        "no fragments at all would make every comparison above vacuous"
    );
}
