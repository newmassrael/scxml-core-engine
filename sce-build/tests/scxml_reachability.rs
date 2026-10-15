//! Statechart graph reachability validation.
//!
//! Drives the wire-layer entry [`sce_build::compile_scxml_lang_typed`]
//! end-to-end so the validator exercises the same path codegen does
//! (XInclude expansion, parser, analyzer, `guard_static_generatable`,
//! and finally `scxml_reachability::validate`). The validator's
//! module-internal unit tests in `sce-build/src/scxml_reachability.rs`
//! cover the BFS algorithm directly with hand-built `SCXMLModel`
//! values; this file pins the contract that the BFS fires through the
//! real compile pipeline.
//!
//! Cases:
//!
//! - **Positive (history default preserves reachability)** — a
//!   document whose only entry to `armed` is through a history
//!   pseudostate's `default_target` compiles clean (W3C SCXML §3.10).
//! - **Negative 1 (`scxml/unreachable-state`)** — an orphan
//!   `<final>` declared at top level with no incoming transition
//!   and not the document `initial` is rejected. The orphan carries
//!   no `<transition>` children so the bare state-level form fires.
//! - **Negative 2 (`scxml/dead-transition`)** — an orphan `<state>`
//!   that contains a `<transition>` is rejected with the
//!   per-transition variant (the (source, target) pair points the
//!   author at the concrete element to repair).
//!
//! Mirrors the per-test tempdir layout of
//! `tests/cross_kind_typed_binding.rs` (NL→IR Item 2): every fixture
//! is staged into a fresh `tempdir` so the suite stays
//! self-contained.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::compile_scxml_lang_typed;
use sce_build::forge::error::ForgeError;
use sce_build::generator::Language;
use sce_build::{find_template_dir_for, scxml_semantic::ScxmlSemanticError};

fn write_fixture(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    fs::write(&path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn compile_positive(dir: &Path, scxml_name: &str) {
    let scxml_path = dir.join(scxml_name);
    let template_dir = find_template_dir_for(Language::Rust);
    compile_scxml_lang_typed(scxml_path.to_str().unwrap(), &template_dir, Language::Rust)
        .expect("reachability validator must accept this document");
}

fn compile_expect_err(
    dir: &Path,
    scxml_name: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    let scxml_path = dir.join(scxml_name);
    let template_dir = find_template_dir_for(Language::Rust);
    match compile_scxml_lang_typed(scxml_path.to_str().unwrap(), &template_dir, Language::Rust) {
        Ok(_) => panic!("reachability validator must reject {scxml_name}"),
        Err(e) => e,
    }
}

#[test]
fn positive_history_default_seeds_reachability() {
    // The document `initial="h"` names a history pseudostate; the
    // only path to `armed` is through `h`'s default_target per W3C
    // SCXML §3.10. Reachability's history-redirect rule must apply
    // so this document compiles clean.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "history_default.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="history_default" initial="parent">
  <state id="parent" initial="h">
    <history id="h" type="shallow">
      <transition target="armed"/>
    </history>
    <state id="armed">
      <transition event="done" target="done_state"/>
    </state>
    <final id="done_state"/>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "history_default.scxml");
}

#[test]
fn orphan_final_without_transitions_emits_unreachable_state() {
    // `<final id="ghost"/>` sits at top level with no incoming
    // transition and is not the document `initial`. The orphan
    // carries no `<transition>` children, so the bare state-level
    // variant fires (the per-transition form outranks it only when
    // the state has at least one transition).
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "orphan_final.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="orphan_final" initial="idle">
  <state id="idle">
    <transition event="done" target="done_state"/>
  </state>
  <final id="done_state"/>
  <final id="ghost"/>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "orphan_final.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::UnreachableState { state_id, .. } => {
                assert_eq!(state_id, "ghost");
            }
            other => panic!("expected UnreachableState variant, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn orphan_state_with_transition_emits_dead_transition() {
    // `<state id="ghost">` is unreachable but carries a transition
    // back to `idle` — the per-transition variant takes priority so
    // the diagnostic names the concrete (source, target) edge.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "dead_transition.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="dead_transition" initial="idle">
  <state id="idle">
    <transition event="done" target="done_state"/>
  </state>
  <state id="ghost">
    <transition event="never" target="idle"/>
  </state>
  <final id="done_state"/>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "dead_transition.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::DeadTransition { state, target, .. } => {
                assert_eq!(state, "ghost");
                assert_eq!(target, "idle");
            }
            other => panic!("expected DeadTransition variant, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn compound_ancestor_transition_reaches_its_target() {
    // W3C SCXML §3.13 — an entered compound state's own transitions
    // belong to the active configuration's transition set (the
    // selection walk climbs the ancestor chain). The BFS seeds at the
    // deep-resolved leaf, so it reaches `s0` only as an ancestor of
    // `s01`; if ancestors are marked reached without being walked,
    // `s0`'s catch-all edge to `fail` is never followed and `fail`
    // reports as an orphan.
    //
    // This is the dominant shape in the W3C IRP suite: a compound
    // state carrying `<transition event="timeout" target="fail"/>`
    // beside its child states.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "ancestor_edge.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="ancestor_edge" initial="s0">
  <state id="s0" initial="s01">
    <transition event="timeout" target="fail"/>
    <state id="s01">
      <transition event="go" target="pass"/>
    </state>
  </state>
  <final id="pass"/>
  <final id="fail"/>
</scxml>
"#,
    );
    compile_positive(dir.path(), "ancestor_edge.scxml");
}

#[test]
fn parallel_sibling_region_reached_through_an_entered_region() {
    // W3C SCXML §3.4 — entering any region of a `<parallel>` enters
    // the parallel, and entering the parallel enters every region.
    // Here only `region_a` is named as a transition target; `region_b`
    // is reachable solely through that implication. A walker that
    // marks the `<parallel>` ancestor reached without walking it never
    // runs the all-regions cascade and reports `region_b` as dead.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "parallel_via_region.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="parallel_via_region" initial="idle">
  <state id="idle">
    <transition event="go" target="region_a"/>
  </state>
  <parallel id="par">
    <state id="region_a">
      <transition event="back" target="idle"/>
    </state>
    <state id="region_b">
      <transition event="back" target="idle"/>
    </state>
  </parallel>
</scxml>
"#,
    );
    compile_positive(dir.path(), "parallel_via_region.scxml");
}

#[test]
fn parallel_all_children_stay_reachable() {
    // W3C SCXML §3.4 — entering `<parallel>` enters every direct
    // non-history child. Region B has no incoming transition and is
    // not parent's `initial`; reachability must still recognise it
    // through the parallel-cascade rule.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "parallel_regions.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="parallel_regions" initial="par">
  <parallel id="par">
    <state id="region_a">
      <transition event="tick" target="region_a"/>
    </state>
    <state id="region_b">
      <transition event="tick" target="region_b"/>
    </state>
  </parallel>
</scxml>
"#,
    );
    compile_positive(dir.path(), "parallel_regions.scxml");
}

// The message of an orphan that is an alternative of an entered sibling. `message` is prose
// (SCE_ERROR_CONTRACT.md §1: not machine-parsed, not part of `id`), so these pin the words an
// author is told, not a wire shape.

fn orphan_message(dir: &Path, scxml_name: &str) -> String {
    compile_expect_err(dir, scxml_name).error.to_string()
}

#[test]
fn a_top_level_sibling_of_the_initial_state_is_told_to_use_a_parallel() {
    // Two top-level states are alternatives (§scxml-3.3): only `a` is entered. An author who
    // meant them to run side by side gets `b` as an orphan, and the bare "unreachable" names
    // neither the sibling nor the way out.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "siblings.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="siblings" initial="a">
  <state id="a"><transition event="go" target="a"/></state>
  <state id="b"/>
</scxml>
"#,
    );
    let said = orphan_message(dir.path(), "siblings.scxml");
    assert!(said.contains("'b'"), "{said}");
    assert!(said.contains("sibling of 'a'"), "{said}");
    assert!(said.contains("top-level states are alternatives"), "{said}");
    assert!(said.contains("under one `<parallel>`"), "{said}");
}

#[test]
fn a_sibling_state_under_a_compound_parent_is_told_to_make_the_parent_a_parallel() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "compound_siblings.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="compound_siblings" initial="top">
  <state id="top" initial="left">
    <state id="left"><transition event="go" target="left"/></state>
    <state id="right"/>
  </state>
</scxml>
"#,
    );
    let said = orphan_message(dir.path(), "compound_siblings.scxml");
    assert!(said.contains("'right'"), "{said}");
    assert!(said.contains("sibling of 'left'"), "{said}");
    assert!(said.contains("sibling states are alternatives"), "{said}");
    assert!(said.contains("make their parent a `<parallel>`"), "{said}");
    assert!(!said.contains("top-level"), "{said}");
}

#[test]
fn the_dead_transition_form_carries_the_same_advice() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "dead_sibling.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="dead_sibling" initial="a">
  <state id="a"><transition event="go" target="a"/></state>
  <state id="b"><transition event="never" target="a"/></state>
</scxml>
"#,
    );
    let said = orphan_message(dir.path(), "dead_sibling.scxml");
    assert!(
        said.contains("Transition in unreachable state 'b'"),
        "{said}"
    );
    assert!(said.contains("sibling of 'a'"), "{said}");
}

#[test]
fn the_first_entered_sibling_in_document_order_is_the_one_named() {
    // `x` and `y` are both entered (`y` is `initial`, `x` is its transition target), `z` is
    // the orphan: the message names the same sibling on every run.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "named_sibling.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="named_sibling" initial="y">
  <state id="x"><transition event="go" target="x"/></state>
  <state id="y"><transition event="go" target="x"/></state>
  <state id="z"/>
</scxml>
"#,
    );
    let said = orphan_message(dir.path(), "named_sibling.scxml");
    assert!(said.contains("sibling of 'x'"), "{said}");
}

#[test]
fn a_document_that_wraps_the_regions_in_a_parallel_is_accepted() {
    // The repair the message names.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "wrapped.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="wrapped" initial="all">
  <parallel id="all">
    <state id="a"><transition event="go" target="a"/></state>
    <state id="b"><transition event="go" target="b"/></state>
  </parallel>
</scxml>
"#,
    );
    compile_positive(dir.path(), "wrapped.scxml");
}
