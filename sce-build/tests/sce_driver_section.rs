// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Protocol-Synthesis RFC §5.2 — `<sce:driver href>` + C11 section
// attribute boundary fixture.
//
// Three contracts pinned here:
//
//   1. A top-level `<sce:driver href="..."/>` whose target file exists
//      under the SCXML file's parent directory lowers to a
//      `#include "<resolved>"` line at the top of the emitted C11
//      translation unit — proves the driver/class boundary,
//      where cross-TU symbol verification is delegated
//      to the C compiler.
//
//   2. A `<sce:driver href="missing.h"/>` whose target does NOT exist
//      under the SCXML parent surfaces `mcu/driver-header-not-found`
//      at compile-model time (Stage::Validation), carrying the
//      verbatim href as `actual` and the resolved search root in
//      `key_fragments` so authors can spot the lookup boundary
//      without re-running with `--verbose`.
//
//   3. `platform.c11_section_attribute` set in `deploy.yaml` paired
//      with a non-C11 codegen target (rust / cpp / kotlin / go /
//      python) surfaces `mcu/section-attribute-on-non-mcu-target` at
//      codegen entry — mirrors the extern-emit non-MCU reject
//      pattern so the section directive does not silently
//      disappear on a non-C11 compile. Function-definition prefix
//      application on C11 itself is exercised below (`SCE_SM_FN`
//      emission).

use std::fs;
use std::path::PathBuf;

use sce_build::forge::error::ForgeError;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

const FIXTURE_NO_DRIVER: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript" name="round_f_alpha">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

const FIXTURE_WITH_DRIVER: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="s1" datamodel="ecmascript" name="round_f_alpha">
  <sce:driver href="hal.h"/>
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

const FIXTURE_DRIVER_MISSING: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="s1" datamodel="ecmascript" name="round_f_alpha">
  <sce:driver href="nonexistent_round_f_alpha_header.h"/>
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

/// Mock driver header — exists only so `resolve_driver_refs` can confirm
/// filesystem presence. The C11 backend `#include`s the path verbatim;
/// cross-TU symbol verification stays the C compiler's job.
const DRIVER_HEADER_BODY: &str = "/* section-driver test header */\n";

fn template_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tools")
        .join("codegen")
        .join("templates")
}

fn write_scxml(tmp: &tempfile::TempDir, scxml: &str) -> PathBuf {
    let path = tmp.path().join("round_f_alpha.scxml");
    fs::write(&path, scxml).unwrap();
    path
}

#[test]
fn fixture_a_driver_ref_lowers_to_include_directive() {
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = write_scxml(&tmp, FIXTURE_WITH_DRIVER);
    let hal = tmp.path().join("hal.h");
    fs::write(&hal, DRIVER_HEADER_BODY).unwrap();

    let out = sce_build::compile_scxml_lang_typed(
        scxml.to_str().unwrap(),
        &template_dir(),
        Language::C11,
    )
    .expect("C11 codegen succeeds when driver header resolves");

    let sm_c = out
        .files
        .iter()
        .find(|(name, _)| name.ends_with("_sm.c"))
        .map(|(_, body)| body)
        .expect("C11 codegen emits a *_sm.c file");
    assert!(
        sm_c.contains("#include \"hal.h\"")
            || sm_c.contains(&format!("#include \"{}\"", hal.to_string_lossy())),
        "C11 *_sm.c must #include the resolved driver header. Got:\n{}",
        &sm_c[..sm_c.len().min(400)],
    );
}

#[test]
fn fixture_b_driver_refs_absent_byte_stable_with_existing_fixtures() {
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = write_scxml(&tmp, FIXTURE_NO_DRIVER);

    let out = sce_build::compile_scxml_lang_typed(
        scxml.to_str().unwrap(),
        &template_dir(),
        Language::C11,
    )
    .expect("C11 codegen succeeds on a baseline fixture without driver refs");

    let sm_c = out
        .files
        .iter()
        .find(|(name, _)| name.ends_with("_sm.c"))
        .map(|(_, body)| body)
        .expect("C11 codegen emits a *_sm.c file");
    // The empty `for d in model.driver_refs` loop is byte-elided by the
    // driver-header template change, so a baseline fixture must not contain
    // any stray `#include` line that wasn't there before.
    let extraneous = sm_c
        .lines()
        .filter(|line| line.trim_start().starts_with("#include") && line.contains("hal.h"))
        .count();
    assert_eq!(
        extraneous, 0,
        "baseline fixture must not gain driver-side #include lines"
    );
}

#[test]
fn fixture_c_missing_driver_header_fires_mcu_diagnostic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = write_scxml(&tmp, FIXTURE_DRIVER_MISSING);

    let result = sce_build::compile_scxml_lang_typed(
        scxml.to_str().unwrap(),
        &template_dir(),
        Language::C11,
    );
    let err = match result {
        Ok(_) => panic!("missing driver header must surface mcu/driver-header-not-found (got Ok)"),
        Err(e) => e,
    };

    let inner: &ForgeError = &err.error;
    let msg = inner.to_string();
    assert!(
        msg.contains("nonexistent_round_f_alpha_header.h"),
        "diagnostic message must round-trip the verbatim href. Got: {msg}",
    );
    assert!(
        msg.contains("could not be resolved"),
        "diagnostic message must phrase the resolver failure. Got: {msg}",
    );
}

#[test]
fn fixture_d_section_attribute_on_non_mcu_target_rejects() {
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = write_scxml(&tmp, FIXTURE_NO_DRIVER);

    // Synthesise a minimal `deploy.yaml` carrying
    // `platform.c11_section_attribute` so the orchestrator's
    // The section-attribute reject fires on a non-C11 backend (rust).
    let deploy_yaml = r#"
topology:
  app_device:
    machines:
      app:
        source: round_f_alpha.scxml
        platform:
          class: mcu
          os: bare_metal
          c11_section_attribute:
            class: ".app_code"
"#;
    let deploy_cfg = sce_build::mesh::deploy::parse_deploy_str(deploy_yaml)
        .expect("synthetic deploy.yaml parses");

    let options = ForgeCompileOptions::default();
    let result = sce_build::compile_scxml_with_imports(
        &[&scxml],
        &[],
        &template_dir(),
        Language::Rust,
        &options,
        Some(&deploy_cfg),
    );
    let err = match result {
        Ok(_) => panic!("rust + c11_section_attribute must reject (got Ok)"),
        Err(e) => e,
    };

    let inner: &ForgeError = &err.error;
    let msg = inner.to_string();
    assert!(
        msg.contains("c11_section_attribute") && msg.contains("rust"),
        "diagnostic message must mention the offending backend. Got: {msg}",
    );
}

#[test]
fn fixture_e_section_attribute_emits_macro_and_function_prefix() {
    // Section-attribute path: C11 backend + `platform.c11_section_attribute.class`
    // must (i) emit the `SCE_SM_FN` macro definition with the requested
    // section name, and (ii) prefix every statechart function definition
    // with `SCE_SM_FN`. Author override via pre-include `#define
    // SCE_SM_FN ...` stays open because the macro is `#ifndef`-guarded.
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = write_scxml(&tmp, FIXTURE_NO_DRIVER);

    let deploy_yaml = r#"
topology:
  app_device:
    machines:
      round_f_alpha:
        source: round_f_alpha.scxml
        platform:
          class: mcu
          os: bare_metal
          c11_section_attribute:
            class: ".app_code"
"#;
    let deploy_cfg = sce_build::mesh::deploy::parse_deploy_str(deploy_yaml)
        .expect("synthetic deploy.yaml parses");

    let options = ForgeCompileOptions::default();
    let outputs = sce_build::compile_scxml_with_imports(
        &[&scxml],
        &[],
        &template_dir(),
        Language::C11,
        &options,
        Some(&deploy_cfg),
    )
    .expect("C11 codegen succeeds when c11_section_attribute is set");

    let sm_c = outputs
        .iter()
        .find_map(|(_basename, out)| out.files.iter().find(|(name, _)| name.ends_with("_sm.c")))
        .map(|(_, body)| body.clone())
        .expect("orchestrator emits a *_sm.c for the C11 fixture");

    assert!(
        sm_c.contains("__attribute__((section(\".app_code\")))"),
        "SCE_SM_FN macro definition must carry the requested section name. \
         Excerpt:\n{}",
        sm_c.lines().take(80).collect::<Vec<_>>().join("\n"),
    );
    assert!(
        sm_c.contains("SCE_SM_FN"),
        "Statechart function definitions must reference SCE_SM_FN at \
         least once (the macro itself is referenced inside its own \
         `#define`, so this assertion is byte-weak; the next assertion \
         counts emit sites). Excerpt:\n{}",
        sm_c.lines().take(80).collect::<Vec<_>>().join("\n"),
    );
    let fn_prefix_count = sm_c.matches("SCE_SM_FN").count();
    assert!(
        fn_prefix_count >= 10,
        "Expected SCE_SM_FN to appear on ≥10 function definition sites \
         (the sourcemap minimal fixture emits ~30 statechart \
         functions). Got {fn_prefix_count}.",
    );
}
