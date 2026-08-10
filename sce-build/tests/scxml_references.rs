//! Statechart state-reference resolution.
//!
//! Drives the wire-layer entry [`sce_build::compile_scxml_lang_typed`]
//! end-to-end so the validator exercises the same path codegen does
//! (XInclude expansion, parser, analyzer, `guard_static_generatable`,
//! and finally `scxml_references::validate`). Sibling file:
//! `sce-build/tests/scxml_reachability.rs`.
//!
//! Every negative case below produced **uncompilable target code** with
//! `exit 0` before this validator existed: the emitted state machine
//! assigns `<Machine>State::<Variant>` for a target the `State` enum
//! never declares, so the document passes `check`, passes `generate`,
//! and fails in the consumer's compiler. That is the contract this
//! file pins — SCE must never answer "this document lowers to
//! <language>" and then emit code that language rejects.
//!
//! W3C SCXML reference-resolution rules covered:
//!
//! - **§3.5 `transition/@target`** — every whitespace-separated token
//!   must name a `<state>` / `<parallel>` / `<final>` / `<history>`
//!   declared in the document.
//! - **§3.3 `state/@initial`** — every token must name a child of the
//!   owning compound state.
//! - **§3.6 `<initial>`** — the initial element's transition target
//!   must resolve.
//! - **§3.10.2 `<history>`** — a history pseudostate carries a single
//!   unconditional `<transition>` naming the default configuration.
//!   The child is required; without it the pseudostate can never be
//!   entered.
//! - **§3.10.2 (target side)** — the history default transition's own
//!   target must resolve like any other transition target.

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
        .expect("reference validator must accept this document");
}

fn compile_expect_err(
    dir: &Path,
    scxml_name: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    let scxml_path = dir.join(scxml_name);
    let template_dir = find_template_dir_for(Language::Rust);
    match compile_scxml_lang_typed(scxml_path.to_str().unwrap(), &template_dir, Language::Rust) {
        Ok(_) => panic!(
            "reference validator must reject {scxml_name} — it lowers to \
             a `State::<Variant>` the generated enum does not declare"
        ),
        Err(e) => e,
    }
}

/// Unwrap to the SCXML semantic variant, panicking with the actual
/// error when the document was rejected for an unrelated reason.
fn scxml_err(
    err: &sce_build::forge::error::Located<sce_build::forge::error::ForgeError>,
) -> &ScxmlSemanticError {
    match &err.error {
        ForgeError::Scxml(boxed) => boxed.as_ref(),
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn transition_target_naming_no_state_is_rejected() {
    // W3C SCXML §3.5. `target="ghost_state"` names nothing in the
    // document. Pre-fix behaviour: `check` reported `status: ok` for
    // all six backends and the Rust emitter wrote
    // `BadTargetState::GhostState` — a variant absent from the enum.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "ghost_target.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="ghost_target" initial="idle">
  <state id="idle">
    <transition event="go" target="ghost_state"/>
    <transition event="fin" target="done_state"/>
  </state>
  <final id="done_state"/>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "ghost_target.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::TransitionTargetUnknown {
            state,
            target,
            available,
        } => {
            assert_eq!(state, "idle");
            assert_eq!(target, "ghost_state");
            // The candidate list feeds `Fix::ReplaceOneOf` on the wire;
            // an empty list suppresses the fix entirely, so the
            // producer must populate it.
            assert!(
                available.contains(&"idle".to_string()),
                "available must list declared states for the ReplaceOneOf fix, got {available:?}"
            );
        }
        other => panic!("expected TransitionTargetUnknown, got {other:?}"),
    }
}

#[test]
fn multi_target_transition_rejects_the_unresolved_token() {
    // W3C SCXML §3.13 multi-target parallel entry: the attribute is a
    // whitespace-separated list and *every* token must resolve. A
    // validator that only inspected the first token would accept this.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "multi_target.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="multi_target" initial="idle">
  <state id="idle">
    <transition event="go" target="par ghost_region"/>
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
    let err = compile_expect_err(dir.path(), "multi_target.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::TransitionTargetUnknown { target, .. } => {
            assert_eq!(
                target, "ghost_region",
                "the unresolved token must be named, not the whole attribute"
            );
        }
        other => panic!("expected TransitionTargetUnknown, got {other:?}"),
    }
}

#[test]
fn compound_initial_naming_no_child_is_rejected() {
    // W3C SCXML §3.3. `<state id="outer" initial="ghost_child">` names
    // a state that does not exist. Pre-fix the emitter wrote
    // `State::GhostChild` into the initial-entry chain.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "ghost_initial.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="ghost_initial" initial="entry">
  <state id="entry">
    <transition event="enter" target="real_child"/>
  </state>
  <state id="outer" initial="ghost_child">
    <state id="real_child">
      <transition event="go" target="done_state"/>
    </state>
  </state>
  <final id="done_state"/>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "ghost_initial.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::InitialStateUnknown {
            state_id, scope, ..
        } => {
            assert_eq!(state_id, "ghost_child");
            assert_eq!(
                *scope,
                sce_build::scxml_semantic::InitialStateScope::CompoundState {
                    parent_id: "outer".to_string()
                },
                "the compound scope must name the owning state so repair \
                 candidates can be scoped to its children"
            );
        }
        other => panic!("expected InitialStateUnknown, got {other:?}"),
    }
}

#[test]
fn initial_element_target_naming_no_state_is_rejected() {
    // W3C SCXML §3.6 — `<initial>` carries a single transition whose
    // target must resolve. The parser folds this into the owning
    // state's `initial`, so the compound-scope diagnostic fires.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "ghost_initial_elem.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="ghost_initial_elem" initial="entry">
  <state id="entry">
    <transition event="enter" target="real_child"/>
  </state>
  <state id="outer">
    <initial>
      <transition target="ghost_init"/>
    </initial>
    <state id="real_child">
      <transition event="go" target="done_state"/>
    </state>
  </state>
  <final id="done_state"/>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "ghost_initial_elem.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::InitialStateUnknown { state_id, .. } => {
            assert_eq!(state_id, "ghost_init");
        }
        other => panic!("expected InitialStateUnknown, got {other:?}"),
    }
}

#[test]
fn history_without_default_transition_is_rejected() {
    // W3C SCXML §3.10.2 requires the single unconditional
    // `<transition>` child. Pre-fix the parser dropped such a history
    // from the model entirely (`if !default_target.is_empty()`), so
    // `target="resume"` fell through to `State::Resume` — a variant
    // the enum never declares, because history pseudostates are not
    // states.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "history_no_default.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="history_no_default" initial="session">
  <state id="session" initial="opening">
    <state id="opening">
      <transition event="next" target="running"/>
      <transition event="away" target="parked"/>
    </state>
    <state id="running">
      <transition event="next" target="opening"/>
      <transition event="away" target="parked"/>
    </state>
    <history id="resume" type="shallow"/>
  </state>
  <state id="parked">
    <transition event="back" target="resume"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "history_no_default.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::HistoryDefaultTransitionMissing {
            history_id,
            parent_id,
            available,
        } => {
            assert_eq!(history_id, "resume");
            assert_eq!(
                parent_id, "session",
                "the owning compound state scopes the legal default targets"
            );
            // The default configuration must be one of the history
            // parent's children (§3.10.2 "descendants of the
            // containing state"), so the candidate list is the repair
            // surface an authoring tool needs.
            assert!(
                available.contains(&"opening".to_string())
                    && available.contains(&"running".to_string()),
                "available must list the parent's children, got {available:?}"
            );
        }
        other => panic!("expected HistoryDefaultTransitionMissing, got {other:?}"),
    }
}

#[test]
fn history_without_default_transition_is_rejected_even_when_untargeted() {
    // §3.10.2 is a declaration rule, not a use rule: the child is
    // required whether or not any transition names the pseudostate.
    // A use-site-only check would accept this document and leave a
    // dead pseudostate in the model.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "history_no_default_unused.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="history_no_default_unused" initial="session">
  <state id="session" initial="opening">
    <state id="opening">
      <transition event="go" target="running"/>
    </state>
    <state id="running">
      <transition event="go" target="opening"/>
    </state>
    <history id="resume" type="deep"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "history_no_default_unused.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::HistoryDefaultTransitionMissing { history_id, .. } => {
            assert_eq!(history_id, "resume");
        }
        other => panic!("expected HistoryDefaultTransitionMissing, got {other:?}"),
    }
}

#[test]
fn history_default_target_naming_no_state_is_rejected() {
    // The history default transition's target is a transition target
    // like any other (§3.10.2 + §3.5). Pre-fix it reached the emitter
    // unresolved and produced `State::GhostLeaf`.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "history_ghost_default.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="history_ghost_default" initial="session">
  <state id="session" initial="opening">
    <state id="opening">
      <transition event="away" target="parked"/>
    </state>
    <history id="resume" type="shallow">
      <transition target="ghost_leaf"/>
    </history>
  </state>
  <state id="parked">
    <transition event="back" target="resume"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "history_ghost_default.scxml");
    match scxml_err(&err) {
        ScxmlSemanticError::TransitionTargetUnknown { state, target, .. } => {
            assert_eq!(
                state, "resume",
                "the history pseudostate owns the offending transition"
            );
            assert_eq!(target, "ghost_leaf");
        }
        other => panic!("expected TransitionTargetUnknown, got {other:?}"),
    }
}

#[test]
fn positive_history_target_with_default_compiles() {
    // The accepting counterpart of
    // `history_without_default_transition_is_rejected`: a history
    // pseudostate IS a legal transition target (§3.5), so the
    // validator must not reject the well-formed shape. Without this
    // case a validator that rejected every history reference would
    // pass the negative tests.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "history_ok.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="history_ok" initial="session">
  <state id="session" initial="opening">
    <state id="opening">
      <transition event="next" target="running"/>
      <transition event="away" target="parked"/>
    </state>
    <state id="running">
      <transition event="next" target="opening"/>
      <transition event="away" target="parked"/>
    </state>
    <history id="resume" type="shallow">
      <transition target="opening"/>
    </history>
  </state>
  <state id="parked">
    <transition event="back" target="resume"/>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "history_ok.scxml");
}

#[test]
fn positive_multi_target_parallel_entry_compiles() {
    // §3.13 multi-target entry with every token resolving. Guards
    // against a validator that mis-splits the attribute and rejects
    // the legal shape.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "multi_target_ok.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="multi_target_ok" initial="idle">
  <state id="idle">
    <transition event="go" target="region_a region_b"/>
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
    compile_positive(dir.path(), "multi_target_ok.scxml");
}

#[test]
fn positive_multi_target_naming_a_history_compiles() {
    // The only production path on which a history id survives to the
    // validator. `parser::resolve_history_targets` rewrites a
    // transition whose *entire* `target` is a history id to that
    // history's default configuration, so the single-target shape never
    // reaches the reference walk carrying a pseudostate id. A
    // multi-target attribute (§3.13) does not match that rewrite, so
    // the raw `ha` token arrives here — and a validator whose legal set
    // omitted history pseudostates would reject this legal document.
    //
    // Measured: with `resolves()` narrowed to `model.states`, this
    // document fails with "references non-existent target state 'ha'".
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "multi_hist.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="multi_hist" initial="par">
  <parallel id="par">
    <state id="ra" initial="ra1">
      <state id="ra1">
        <transition event="go" target="ra2"/>
        <transition event="park" target="parked"/>
      </state>
      <state id="ra2">
        <transition event="go" target="ra1"/>
        <transition event="park" target="parked"/>
      </state>
      <history id="ha" type="shallow">
        <transition target="ra1"/>
      </history>
    </state>
    <state id="rb" initial="rb1">
      <state id="rb1">
        <transition event="go" target="rb2"/>
      </state>
      <state id="rb2">
        <transition event="go" target="rb1"/>
      </state>
    </state>
  </parallel>
  <state id="parked">
    <transition event="resume" target="ha rb1"/>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "multi_hist.scxml");
}

#[test]
fn positive_targetless_transition_compiles() {
    // §5.9.2 targetless internal transitions carry no `target`
    // attribute at all. A validator that treated the empty string as
    // an unresolved reference would reject every executable-only
    // transition in the corpus.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "targetless_ok.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="targetless_ok" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="counter" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="tick">
      <assign location="counter" expr="counter + 1"/>
    </transition>
    <transition event="go" target="done_state"/>
  </state>
  <final id="done_state"/>
</scxml>
"#,
    );
    compile_positive(dir.path(), "targetless_ok.scxml");
}
