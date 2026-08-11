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
//! - **Positive (`exhaustiveness_opt_out`)** — same gap shape with
//!   `sce:exhaustive="false"` on the parent. Accept.
//! - **Positive (`exhaustiveness_disjoint_protocol`)** — the
//!   W3C-IRP-style protocol-stage pattern with disjoint event
//!   vocabularies across siblings. No common ground exists, so the
//!   heuristic stays silent. Accept.
//! - **Negative (`sce_exhaustive_invalid_value`)** — non-`true`/
//!   `false` literal on the opt-out attribute rejects via
//!   `validation/invalid-attribute`.

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
fn opt_out_attribute_accepted() {
    // Same gap shape with `sce:exhaustive="false"` on the parent.
    // Accept regardless of the validator's heuristic.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "opt_out.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="opt_out" initial="dispatch">
  <state id="dispatch" initial="idle" sce:exhaustive="false">
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
    compile_positive(dir.path(), "opt_out.scxml");
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

#[test]
fn invalid_optout_value_rejected() {
    // `sce:exhaustive` accepts only the literal `"true"` and
    // `"false"`. Any other value rejects via
    // `validation/invalid-attribute` so the opt-out cannot be
    // silently mis-spelled.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "invalid_optout.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="invalid_optout" initial="parent">
  <state id="parent" initial="a" sce:exhaustive="no">
    <state id="a">
      <transition event="go" target="b"/>
    </state>
    <state id="b">
      <transition event="go" target="a"/>
    </state>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "invalid_optout.scxml");
    let diags = {
        use sce_build::forge::diagnostic::ToDiagnostics;
        err.error.to_diagnostics()
    };
    let code_str = serde_json::to_string(&diags[0].code).unwrap();
    assert_eq!(code_str, "\"validation/invalid-attribute\"");
    assert_eq!(diags[0].actual.as_deref(), Some("no"));
}

/// A compound with more than one gap says so.
///
/// `sce:exhaustive="false"` is the only escape hatch and it covers the
/// PARENT, so an author reaching for it is deciding about every gap
/// underneath. The validator stops at the first violation — the wire
/// layer carries one record — and that used to mean the message named
/// one event while the annotation would silence three. This tree's own
/// `examples/doom_wasm/scxml/combo_state.scxml` is that case: its
/// comment reasons about `berserk_activate` while `combo_timeout` and
/// `berserk_timeout` are gaps too, and nothing ever showed them.
#[test]
fn a_compound_with_several_gaps_names_the_ones_the_optout_would_also_cover() {
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
                    "the sibling gap the opt-out would also silence is missing",
                );
            }
            other => panic!("expected NonExhaustiveEventHandling, got {other:?}"),
        },
        other => panic!("expected an SCXML semantic error, got {other:?}"),
    }

    let text = err.error.to_string();
    assert!(
        text.contains("would also silence"),
        "message does not warn what the annotation covers: {text}",
    );
}
