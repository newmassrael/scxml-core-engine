//! Event-set exhaustiveness validation.
//!
//! Drives the wire-layer entry [`sce_build::compile_scxml_lang_typed`]
//! end-to-end so the validator exercises the full compile path
//! (XInclude expansion, parser, analyzer, `guard_static_generatable`,
//! `scxml_reachability::validate`, then the exhaustiveness walker). The
//! validator's module-internal unit tests in
//! `sce-build/src/scxml_exhaustiveness.rs` cover the matching
//! semantics directly with hand-built `SCXMLModel` values; this file
//! pins the contract that the heuristic fires through the real
//! compile pipeline.
//!
//! Cases mirror the user's reference fixture spec:
//!
//! - **Negative (`exhaustiveness_compound_gap`)** — three siblings
//!   share `cmd.stop` as common ground but only two handle
//!   `cmd.start`. The validator rejects with
//!   `scxml/non-exhaustive-event-handling`.
//! - **Positive (`exhaustiveness_parent_fallthrough`)** — same gap
//!   shape but the parent compound state declares a transition
//!   handling `cmd.start` itself, turning the bubble into a
//!   deliberate fallthrough. Accept.
//! - **Positive (`declared`)** — same gap shape with `sce:unhandled`
//!   on the child that leaves the event unhandled. Accept.
//! - **Negative (`later_sibling`)** — the same declaration, plus a
//!   sibling added afterwards that also fails to handle the event and
//!   declares nothing. Rejected, naming only that sibling. This is the
//!   case the withdrawn parent-level opt-out could not express.
//! - **Positive (`exhaustiveness_disjoint_protocol`)** — the
//!   W3C-IRP-style protocol-stage pattern with disjoint event
//!   vocabularies across siblings. No common ground exists, so the
//!   heuristic stays silent. Accept.
//! - **Negative (`contradiction` / `stale`)** — a declaration the
//!   document contradicts, and one that exempts nothing. The two
//!   directions in which a declaration can stop being true.
//! - **Negative (`attribute_shape_rejections`)** — the withdrawn
//!   `sce:exhaustive` and the malformed `sce:unhandled` token forms,
//!   all via `validation/invalid-attribute`.

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
        .expect("exhaustiveness validator must accept this document");
}

fn compile_expect_err(
    dir: &Path,
    scxml_name: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    let scxml_path = dir.join(scxml_name);
    let template_dir = find_template_dir_for(Language::Rust);
    match compile_scxml_lang_typed(scxml_path.to_str().unwrap(), &template_dir, Language::Rust) {
        Ok(_) => panic!("exhaustiveness validator must reject {scxml_name}"),
        Err(e) => e,
    }
}

#[test]
fn compound_gap_rejected() {
    // Three siblings share `cmd.stop` as common ground; `cmd.start`
    // is handled by `idle` and `stopped` but not by `active`, and
    // the parent has no fallthrough. Reject.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "compound_gap.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="compound_gap" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="active">
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "compound_gap.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::NonExhaustiveEventHandling {
                parent,
                event,
                handlers,
                non_handlers,
                also,
            } => {
                assert_eq!(parent, "dispatch");
                assert_eq!(event, "cmd.start");
                assert_eq!(handlers, &vec!["idle".to_string(), "stopped".to_string()]);
                assert_eq!(non_handlers, &vec!["active".to_string()]);
                // One gap in this fixture, so the escape hatch would
                // cover exactly what the message names.
                assert!(also.is_empty(), "unexpected sibling gaps: {also:?}");
            }
            other => panic!("expected NonExhaustiveEventHandling variant, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn parent_fallthrough_accepted() {
    // Same gap shape as the rejection case, but the compound parent
    // itself handles `cmd.start` — W3C SCXML §3.13 bubble semantics
    // turn the per-sibling gap into a deliberate fallthrough.
    // Accept.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "parent_fallthrough.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="parent_fallthrough" initial="dispatch">
  <state id="dispatch" initial="idle">
    <!-- parent absorbs cmd.start regardless of child -->
    <transition event="cmd.start" target="dispatch"/>
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="active">
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "parent_fallthrough.scxml");
}

#[test]
fn unhandled_declaration_accepted() {
    // Same gap shape, declared on the child that actually leaves
    // `cmd.start` unhandled.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "declared.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="declared" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="active" sce:unhandled="cmd.start">
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "declared.scxml");
}

/// The reason the declaration sits on the child rather than the
/// parent: a sibling added after the exemption was written inherits
/// nothing and is judged on its own.
///
/// `active` declares `cmd.start`, which is true of `active`. `draining`
/// arrives later, also fails to handle `cmd.start`, and declares
/// nothing — under the withdrawn parent-level opt-out the compound
/// would still be silent and nobody would ever judge `draining`.
#[test]
fn a_sibling_added_after_the_declaration_is_still_judged() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "later_sibling.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="later_sibling" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="active" sce:unhandled="cmd.start">
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="draining"/>
    </state>
    <state id="draining">
      <transition event="cmd.stop" target="stopped"/>
    </state>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "later_sibling.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::NonExhaustiveEventHandling {
                event,
                non_handlers,
                ..
            } => {
                assert_eq!(event, "cmd.start");
                // `active` declared it; the report tracks only what is
                // left to decide.
                assert_eq!(non_handlers, &vec!["draining".to_string()]);
            }
            other => panic!("expected NonExhaustiveEventHandling variant, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn declaration_contradicted_by_a_transition_rejected() {
    // `active` declares `cmd.stop` unhandled and handles it.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "contradiction.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="contradiction" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="active" sce:unhandled="cmd.stop">
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "contradiction.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::ContradictoryUnhandledDeclaration { state, event } => {
                assert_eq!(state, "active");
                assert_eq!(event, "cmd.stop");
            }
            other => panic!("expected ContradictoryUnhandledDeclaration, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn declaration_that_names_no_gap_rejected() {
    // `active` declares an event no sibling handles either, so the
    // declaration exempts nothing.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "stale.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="stale" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="active" sce:unhandled="cmd.reset">
      <transition event="cmd.start" target="stopped"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "stale.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::StaleUnhandledDeclaration {
                state,
                parent,
                event,
                gaps,
            } => {
                assert_eq!(state, "active");
                assert_eq!(parent, "dispatch");
                assert_eq!(event, "cmd.reset");
                assert!(gaps.is_empty(), "no gap names `active`: {gaps:?}");
            }
            other => panic!("expected StaleUnhandledDeclaration, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

/// A declaration on a child that is not part of the comparison at all.
///
/// `<final>` children and children with no transitions are excluded from
/// the sibling comparison, so they can never be a non-handler and their
/// declarations exempt nothing — even when the event they name is a
/// genuine gap for some other sibling. Staleness is judged per (child,
/// event), not per parent: a mutation weakening it to "is this event a
/// gap anywhere under the parent?" is invisible to every fixture whose
/// declarer happens to be a real non-handler.
#[test]
fn declaration_on_a_child_outside_the_comparison_rejected() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "outside.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="outside" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
      <transition event="cmd.quit" target="done"/>
    </state>
    <state id="active" sce:unhandled="cmd.start">
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <state id="stopped">
      <transition event="cmd.start" target="active"/>
      <transition event="cmd.stop" target="stopped"/>
    </state>
    <final id="done" sce:unhandled="cmd.start"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "outside.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::StaleUnhandledDeclaration {
                state,
                parent,
                event,
                gaps,
            } => {
                assert_eq!(state, "done");
                assert_eq!(parent, "dispatch");
                assert_eq!(event, "cmd.start");
                // `cmd.start` IS a gap under `dispatch` — just not one
                // that names `done`. That is the distinction.
                assert!(gaps.is_empty(), "no gap names `done`: {gaps:?}");
            }
            other => panic!("expected StaleUnhandledDeclaration, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

/// A `<parallel>` sibling carrying transitions is in the comparison on
/// the same terms as a `<state>`, so it needs the same way to declare a
/// deliberate gap.
///
/// The withdrawn parent-level opt-out covered this case for free by
/// covering everything. Replacing it with a per-child declaration means
/// every element that can be a non-handler has to be able to carry one —
/// otherwise the replacement removes expressiveness instead of sharpening
/// it.
#[test]
fn a_parallel_sibling_can_declare_its_gap() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "parallel_declares.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="parallel_declares" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="cmd.start" target="region"/>
      <transition event="cmd.stop" target="idle"/>
    </state>
    <parallel id="region" sce:unhandled="cmd.start">
      <transition event="cmd.stop" target="idle"/>
      <state id="left">
        <transition event="left.tick" target="left"/>
      </state>
      <state id="right">
        <transition event="right.tick" target="right"/>
      </state>
    </parallel>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "parallel_declares.scxml");
}

#[test]
fn disjoint_protocol_stages_accepted() {
    // W3C-IRP-style sequential protocol pattern. Each stage handles
    // its own event family; no common-ground event exists across
    // siblings, so the heuristic stays silent.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "disjoint_protocol.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="disjoint_protocol" initial="conn">
  <state id="conn" initial="connecting">
    <state id="connecting">
      <transition event="connected" target="ready"/>
      <transition event="net.error" target="failed"/>
    </state>
    <state id="ready">
      <transition event="request" target="processing"/>
      <transition event="disconnect" target="connecting"/>
    </state>
    <state id="processing">
      <transition event="response" target="ready"/>
      <transition event="timeout" target="failed"/>
    </state>
    <state id="failed">
      <transition event="retry" target="connecting"/>
    </state>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "disjoint_protocol.scxml");
}

/// Attribute-shape rejections, all `validation/invalid-attribute`.
///
/// The withdrawn `sce:exhaustive` is in this table for a reason that
/// is not tidiness: an unrecognised `sce:` attribute on a statechart
/// element is accepted and ignored, so a document still carrying the
/// old parent-level opt-out would lose its exemption in silence. Being
/// rejected by name is what makes that migration loud.
#[test]
fn attribute_shape_rejections() {
    let cases: &[(&str, &str, &str)] = &[
        // (fixture stem, parent/child attribute text, `actual` payload)
        (
            "withdrawn_optout",
            r#"<state id="parent" initial="a" sce:exhaustive="false">"#,
            "false",
        ),
        (
            "withdrawn_optout_true",
            r#"<state id="parent" initial="a" sce:exhaustive="true">"#,
            "true",
        ),
        (
            "wildcard_token",
            r#"<state id="parent" initial="a" sce:unhandled="go.*">"#,
            "go.*",
        ),
        (
            "universal_wildcard_token",
            r#"<state id="parent" initial="a" sce:unhandled="*">"#,
            "*",
        ),
        (
            "duplicate_token",
            r#"<state id="parent" initial="a" sce:unhandled="go go">"#,
            "go",
        ),
        (
            "empty_declaration",
            r#"<state id="parent" initial="a" sce:unhandled="   ">"#,
            "   ",
        ),
    ];

    for (stem, parent_open, expected_actual) in cases {
        let dir = tempdir().expect("tempdir");
        let name = format!("{stem}.scxml");
        write_fixture(
            dir.path(),
            &name,
            &format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="{stem}" initial="parent">
  {parent_open}
    <state id="a">
      <transition event="go" target="b"/>
    </state>
    <state id="b">
      <transition event="go" target="a"/>
    </state>
  </state>
</scxml>
"#
            ),
        );
        let err = compile_expect_err(dir.path(), &name);
        let diags = {
            use sce_build::forge::diagnostic::ToDiagnostics;
            err.error.to_diagnostics()
        };
        let code_str = serde_json::to_string(&diags[0].code).unwrap();
        assert_eq!(
            code_str, "\"validation/invalid-attribute\"",
            "{stem} rejected under the wrong code"
        );
        assert_eq!(
            diags[0].actual.as_deref(),
            Some(*expected_actual),
            "{stem} named the wrong value"
        );
    }
}

/// A compound with more than one gap says so.
///
/// The validator stops at the first violation — the wire layer carries
/// one record — so without `also` an author repairing a compound pays a
/// build round per gap. This tree's own
/// `examples/doom_wasm/scxml/combo_state.scxml` is the case that
/// motivated it: its comment reasoned about `berserk_activate` while
/// `combo_timeout` and `berserk_timeout` were gaps too, and nothing
/// ever showed them.
#[test]
fn a_compound_with_several_gaps_names_the_others_too() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "many_gaps.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="many_gaps" initial="dispatch">
  <state id="dispatch" initial="idle">
    <state id="idle">
      <transition event="shared" target="active"/>
      <transition event="only_idle" target="active"/>
    </state>
    <state id="active">
      <transition event="shared" target="idle"/>
      <transition event="only_active" target="idle"/>
    </state>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "many_gaps.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::NonExhaustiveEventHandling { event, also, .. } => {
                // Two gaps: whichever is reported, the other must be named.
                let other = if event == "only_idle" {
                    "only_active"
                } else {
                    "only_idle"
                };
                assert_eq!(
                    also,
                    &vec![other.to_string()],
                    "the sibling gap needing its own repair is missing",
                );
            }
            other => panic!("expected NonExhaustiveEventHandling, got {other:?}"),
        },
        other => panic!("expected an SCXML semantic error, got {other:?}"),
    }

    let text = err.error.to_string();
    assert!(
        text.contains("inconsistent too"),
        "message does not report the compound's other gaps: {text}",
    );
}
