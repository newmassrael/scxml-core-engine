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

// ── IAR toolchain arm ────────────────────────────────────────────

/// Compile the emitted translation unit and report every `FUNC` symbol
/// that did not land in `expect_section`. Returns `None` when no C
/// compiler is available.
fn funcs_outside_section(files: &[(String, String)], expect_section: &str) -> Option<Vec<String>> {
    use std::process::Command;
    let cc = resolve("gcc").or_else(|| resolve("cc"))?;
    let readelf = resolve("readelf")?;

    let tmp = tempfile::TempDir::new().unwrap();
    let src = write_unit(tmp.path(), files);
    let obj = tmp.path().join("unit.o");
    let runtime_inc = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("backends/c/runtime/include");
    let out = Command::new(&cc)
        .args(["-std=c11", "-c"])
        .arg("-I")
        .arg(&runtime_inc)
        .arg("-I")
        .arg(tmp.path())
        .arg(&src)
        .arg("-o")
        .arg(&obj)
        .output()
        .expect("run cc");
    assert!(
        out.status.success(),
        "generated C must compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let sections = Command::new(&readelf)
        .args(["-SW"])
        .arg(&obj)
        .output()
        .expect("readelf -SW");
    let sections = String::from_utf8_lossy(&sections.stdout).into_owned();
    let idx = sections
        .lines()
        .find(|l| l.contains(expect_section))
        .and_then(|l| l.split(']').next())
        .and_then(|l| l.rsplit('[').next())
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| panic!("no `{expect_section}` section in the object file:\n{sections}"));

    let syms = Command::new(&readelf)
        .args(["-sW"])
        .arg(&obj)
        .output()
        .expect("readelf -sW");
    let syms = String::from_utf8_lossy(&syms.stdout).into_owned();
    let mut escaped = Vec::new();
    for line in syms.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 8 && f[3] == "FUNC" && f[6] != idx {
            escaped.push(f[7].to_string());
        }
    }
    Some(escaped)
}

/// Locate a toolchain binary, searching past `PATH` into the versioned
/// install directories distributions use. See [`sce_build::toolchain`]
/// for why a `PATH`-only probe silently narrows what these tests check.
fn resolve(tool: &str) -> Option<PathBuf> {
    sce_build::toolchain::locate(tool)
}

/// Render every C11 file for `FIXTURE_NO_DRIVER` with the given
/// section class declared in deploy.yaml. Both the `.c` and its `.h`
/// come back because the translation unit includes its own header —
/// compiling the `.c` alone would fail on the include, not on anything
/// this file is testing.
fn render_c11_with_section_files(class: &str) -> Vec<(String, String)> {
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = write_scxml(&tmp, FIXTURE_NO_DRIVER);
    let deploy_yaml = format!(
        r#"
topology:
  app_device:
    machines:
      round_f_alpha:
        source: round_f_alpha.scxml
        platform:
          class: mcu
          os: bare_metal
          c11_section_attribute:
            class: "{class}"
"#
    );
    let deploy_cfg =
        sce_build::mesh::deploy::parse_deploy_str(&deploy_yaml).expect("deploy.yaml parses");
    let outputs = sce_build::compile_scxml_with_imports(
        &[&scxml],
        &[],
        &template_dir(),
        Language::C11,
        &ForgeCompileOptions::default(),
        Some(&deploy_cfg),
    )
    .expect("C11 codegen succeeds");
    let files: Vec<(String, String)> = outputs
        .iter()
        .flat_map(|(_b, out)| out.files.iter().cloned())
        .collect();
    assert!(
        files.iter().any(|(n, _)| n.ends_with("_sm.c")),
        "a *_sm.c is emitted"
    );
    files
}

/// The `_sm.c` body alone, for the assertions that only read text.
fn render_c11_with_section(class: &str) -> String {
    render_c11_with_section_files(class)
        .into_iter()
        .find(|(n, _)| n.ends_with("_sm.c"))
        .map(|(_, body)| body)
        .expect("a *_sm.c is emitted")
}

/// Write a rendered file set into `dir` and return the `.c` path.
fn write_unit(dir: &std::path::Path, files: &[(String, String)]) -> PathBuf {
    let mut unit = None;
    for (name, body) in files {
        let p = dir.join(name);
        fs::write(&p, body).expect("write generated file");
        if name.ends_with("_sm.c") {
            unit = Some(p);
        }
    }
    unit.expect("a *_sm.c is emitted")
}

#[test]
fn section_placement_selects_the_syntax_the_running_compiler_accepts() {
    // IAR and the GCC family spell function placement differently and
    // neither parses the other's form, so a generated file that
    // committed to one would only build on one. The section NAME is a
    // property of the deployment; the SYNTAX is a property of the
    // compiler, which is why the choice is a `#if` on the compiler's
    // own predefined macro rather than a second deploy.yaml knob that
    // could disagree with the build system.
    let sm_c = render_c11_with_section(".app_code");

    assert!(
        sm_c.contains("#if defined(__IAR_SYSTEMS_ICC__)"),
        "the arm must be selected by IAR's own predefined macro:\n{sm_c}"
    );
    assert!(
        sm_c.contains(r#"#define SCE_SM_FN _Pragma("location=\".app_code\"")"#),
        "IAR arm must place via `#pragma location`, reached through _Pragma \
         so it can live in a macro:\n{sm_c}"
    );
    assert!(
        sm_c.contains(r#"#define SCE_SM_FN __attribute__((section(".app_code")))"#),
        "the GCC/Clang/Keil arm must survive unchanged:\n{sm_c}"
    );
}

#[test]
fn every_emitted_statechart_function_lands_in_the_declared_section() {
    // The macro's documented contract is "every emitted statechart
    // function definition". Two escaped it — `_initial_state` and
    // `_init_impl` — which for an MCU deployment placing code in fast
    // RAM or a protected region is exactly the failure the feature
    // exists to prevent: a stray function outside the region the
    // linker script reserved.
    //
    // Checked against the linker's own view rather than by grepping
    // for the macro, so a function that carries the macro but does not
    // actually get placed would still be caught.
    let files = render_c11_with_section_files(".app_code");
    let Some(escaped) = funcs_outside_section(&files, ".app_code") else {
        eprintln!("SKIP: no gcc/readelf on PATH");
        return;
    };

    // `sce_*` are `static inline` helpers from `sce/types.h`, not
    // emitted statechart functions — they are outside the contract and
    // outside the section by the same token.
    let ours: Vec<&String> = escaped.iter().filter(|n| !n.starts_with("sce_")).collect();
    assert!(
        ours.is_empty(),
        "these emitted statechart functions are outside `.app_code`: {ours:?}"
    );
}

#[test]
fn the_iar_arm_parses_at_every_emission_site() {
    // `#pragma location` applies to the NEXT declaration, so the macro
    // has to sit at the very start of each declaration — not between
    // the storage class and the return type, where it used to sit for
    // most of the ~60 sites. A single site left in the old position
    // would be a pragma bound to the wrong declaration, or a parse
    // error, on IAR.
    //
    // GCC cannot check `location`'s semantics, but it can check that
    // the arm is well-formed C in every position it appears, which is
    // what the normalization actually put at risk.
    use std::process::Command;
    let Some(cc) = resolve("gcc").or_else(|| resolve("cc")) else {
        eprintln!("SKIP: no gcc/cc on PATH");
        return;
    };
    let files = render_c11_with_section_files(".app_code");
    let tmp = tempfile::TempDir::new().unwrap();
    let src = write_unit(tmp.path(), &files);
    let runtime_inc = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("backends/c/runtime/include");

    let out = Command::new(&cc)
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-Wno-unknown-pragmas",
            "-D__IAR_SYSTEMS_ICC__=9",
            "-fsyntax-only",
        ])
        .arg("-I")
        .arg(&runtime_inc)
        .arg("-I")
        .arg(tmp.path())
        .arg(&src)
        .output()
        .expect("run cc");
    assert!(
        out.status.success(),
        "the IAR arm must be well-formed C at every emission site:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_section_name_that_would_escape_the_string_literal_is_rejected() {
    // The name lands in a plain C string on the GCC family and in a
    // string-inside-a-string on IAR. A quote closes one of them, and
    // the IAR form is where it is least visible — the generated file
    // would carry a `_Pragma` whose argument silently ends early.
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
            class: ".app\"code"
"#;
    let deploy_cfg =
        sce_build::mesh::deploy::parse_deploy_str(deploy_yaml).expect("deploy.yaml parses");
    let result = sce_build::compile_scxml_with_imports(
        &[&scxml],
        &[],
        &template_dir(),
        Language::C11,
        &ForgeCompileOptions::default(),
        Some(&deploy_cfg),
    );
    let Err(err) = result else {
        panic!("a quote in the section name must not reach the emitted file");
    };
    let text = err.to_string();
    assert!(
        text.contains("double quote") && text.contains("section name"),
        "diagnostic must name the offending character, got: {text}"
    );
}
