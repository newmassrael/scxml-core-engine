//! Guard analysis validation.
//!
//! Drives the wire-layer entry [`sce_build::compile_scxml_lang_typed`]
//! end-to-end so the guard-analysis validators exercise the full compile
//! path. The validator's module-internal unit tests in
//! `sce-build/src/scxml_guard_analysis.rs` cover the classifier
//! semantics directly with hand-built `SCXMLModel` values; this
//! file pins the contract that the heuristics fire through the
//! real compile pipeline.

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
        .expect("guard-analysis validator must accept this document");
}

fn compile_expect_err(
    dir: &Path,
    scxml_name: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    let scxml_path = dir.join(scxml_name);
    let template_dir = find_template_dir_for(Language::Rust);
    match compile_scxml_lang_typed(scxml_path.to_str().unwrap(), &template_dir, Language::Rust) {
        Ok(_) => panic!("guard-analysis validator must reject {scxml_name}"),
        Err(e) => e,
    }
}

#[test]
fn always_false_literal_rejected() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "always_false_literal.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="always_false_literal" initial="armed">
  <state id="armed">
    <transition event="fire" cond="false" target="armed"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "always_false_literal.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::AlwaysFalseGuard { state, cond } => {
                assert_eq!(state, "armed");
                assert_eq!(cond, "false");
            }
            other => panic!("expected AlwaysFalseGuard, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn always_false_arithmetic_rejected() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "always_false_arith.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="always_false_arith" initial="armed">
  <state id="armed">
    <transition event="fire" cond="1 == 2" target="armed"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "always_false_arith.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::AlwaysFalseGuard { cond, .. } => {
                assert_eq!(cond, "1 == 2");
            }
            other => panic!("expected AlwaysFalseGuard, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn shadowed_by_unconditional_rejected() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "shadowed_by_uncond.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="shadowed_by_uncond" initial="armed">
  <state id="armed">
    <!-- Unconditional fires first, shadows the guarded sibling. -->
    <transition event="fire" target="armed"/>
    <transition event="fire" cond="ctx.ready" target="armed"/>
  </state>
</scxml>
"#,
    );
    let err = compile_expect_err(dir.path(), "shadowed_by_uncond.scxml");
    match &err.error {
        ForgeError::Scxml(boxed) => match boxed.as_ref() {
            ScxmlSemanticError::ShadowedTransition {
                state,
                event,
                shadowing_index,
                shadowed_index,
            } => {
                assert_eq!(state, "armed");
                assert_eq!(event, "fire");
                assert_eq!(*shadowing_index, 0);
                assert_eq!(*shadowed_index, 1);
            }
            other => panic!("expected ShadowedTransition, got {other:?}"),
        },
        other => panic!("expected ForgeError::Scxml, got {other:?}"),
    }
}

#[test]
fn opaque_identifier_guard_accepted() {
    // Author-supplied identifier guards stay opaque to the
    // guard-analysis validator — the runtime data model resolves
    // them, the parser cannot reason about their truth value.
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "opaque_guard.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" datamodel="ecmascript"
       version="1.0" name="opaque_guard" initial="armed">
  <datamodel>
    <data id="ready" expr="false"/>
  </datamodel>
  <state id="armed">
    <transition event="fire" cond="ready" target="armed"/>
  </state>
</scxml>
"#,
    );
    compile_positive(dir.path(), "opaque_guard.scxml");
}
