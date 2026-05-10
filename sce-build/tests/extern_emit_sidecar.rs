// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// watching-zenoh RFC §5.I `<sce:extern>` per-language sidecar emit —
// Atomic C end-to-end fixtures. Each test exercises
// `compile_forge_with_imports` against an inline SCXML carrying one
// or more `<sce:extern>` declarations and asserts on the emitted
// sidecar artifact:
//
//   - Rust: `<snake>_externs.rs` carries an `extern "C" { ... }`
//     block with verbatim Rust sigs.
//   - C11: `<snake>_externs.h` carries `extern <ret> name(<C-translated>)`
//     forward declarations.
//   - Cpp: `<snake>_externs.h` carries `extern "C" <ret> name(...)`
//     prototypes.
//   - Kotlin / Go / Python: rejection via existing
//     `codegen/mcu-class-kind-on-non-mcu-language` family
//     (Q-Call-7 lock).
//
// Sidecar naming follows the existing algorithm/codec/buffer-pool
// sidecar convention (`<snake>_<purpose>.<ext>`).

use sce_build::compile_forge_with_imports;
use sce_build::forge::error::{ForgeError, GenerateError};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};
use std::path::Path;

/// Wrap one or more `<sce:extern>` declarations in a minimal
/// `transform` kind SCXML — same fixture shape used by atomic A/B.
fn fixture_transform_with_externs(extern_decls: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="extern_c_test">
  {extern_decls}
  <datamodel>
    <data id="x" sce:type="uint32" sce:direction="in"/>
    <data id="y" sce:type="uint32" sce:direction="out" expr="x"/>
  </datamodel>
</scxml>
"##
    )
}

fn compile(
    scxml: &str,
    lang: Language,
) -> Result<sce_build::generator::GeneratedOutput, ForgeError> {
    compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("extern_c"),
        lang,
        Path::new("."),
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| e.error)
}

fn find_sidecar<'a>(
    output: &'a sce_build::generator::GeneratedOutput,
    suffix: &str,
) -> Option<&'a String> {
    output
        .files
        .iter()
        .find(|(name, _)| name.ends_with(suffix))
        .map(|(_, code)| code)
}

#[test]
fn rust_emits_extern_c_block_for_baseline_symbols() {
    // C5 (spec §5.E line 1548): the cache-maintenance trio
    // (`sce_dcache_*_by_addr`) is rejected at parse time when authored
    // via `<sce:extern>` because cache calls are FSM-driven from the
    // buffer-pool kind. To exercise the Rust emit shape for a multi-
    // param symbol, this fixture authors `sce_atomic_fetch_add_acq_rel_u32`
    // (signature `(*mut u32, u32) -> u32`) — non-cache, multi-param,
    // covers the same Rust→C type translator code paths the original
    // `sce_dcache_clean_by_addr` exercised.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>
  <sce:extern name="sce_atomic_fetch_add_acq_rel_u32" sig="(*mut u32, u32) -> u32" abi="c"/>"##,
    );
    let output = compile(&scxml, Language::Rust).expect("must compile with externs");
    let sidecar = find_sidecar(&output, "_externs.rs").expect("rust sidecar emitted");

    // Rust emit shape per Q-Call-7: `extern "C" { fn name(p0: T0) -> R; }`.
    assert!(
        sidecar.contains("unsafe extern \"C\" {"),
        "expected `unsafe extern \"C\" {{` block in sidecar:\n{sidecar}",
    );
    assert!(
        sidecar.contains("pub fn sce_atomic_load_acquire_u32(p0: *const u32) -> u32;"),
        "expected atomic_load decl in sidecar:\n{sidecar}",
    );
    assert!(
        sidecar.contains("pub fn sce_atomic_fetch_add_acq_rel_u32(p0: *mut u32, p1: u32) -> u32;"),
        "expected atomic_fetch_add decl in sidecar:\n{sidecar}",
    );
}

#[test]
fn c11_emits_extern_forward_decls() {
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>
  <sce:extern name="sce_atomic_store_release_u32" sig="(*mut u32, u32)" abi="c"/>"##,
    );
    let output = compile(&scxml, Language::C11).expect("must compile with externs");
    let sidecar = find_sidecar(&output, "_externs.h").expect("c11 sidecar emitted");

    // C11 emit shape per Q-Call-7: `extern <ret> name(<C-args>);`.
    assert!(
        sidecar.contains("extern uint32_t sce_atomic_load_acquire_u32(const uint32_t* p0);"),
        "expected atomic_load forward decl in C11 sidecar:\n{sidecar}",
    );
    assert!(
        sidecar.contains("extern void sce_atomic_store_release_u32(uint32_t* p0, uint32_t p1);"),
        "expected atomic_store forward decl in C11 sidecar:\n{sidecar}",
    );
    // Header guard + standard includes per the C11 sidecar template.
    assert!(sidecar.contains("#ifndef SCE_FORGE_"));
    assert!(sidecar.contains("#include <stdint.h>"));
}

#[test]
fn cpp_emits_extern_c_per_decl() {
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_irq_save" sig="() -> irq_state_t" abi="c"/>
  <sce:extern name="sce_irq_restore" sig="(irq_state_t)" abi="c"/>"##,
    );
    let output = compile(&scxml, Language::Cpp).expect("must compile with externs");
    let sidecar = find_sidecar(&output, "_externs.h").expect("cpp sidecar emitted");

    // Cpp emit shape per Q-Call-7: `extern "C" <ret> name(<C-args>);`.
    // Each prototype carries its own `extern "C"` so the sidecar can
    // be included from a C++ header without an extra wrapper.
    assert!(
        sidecar.contains("extern \"C\" irq_state_t sce_irq_save(void);"),
        "expected sce_irq_save proto in cpp sidecar:\n{sidecar}",
    );
    assert!(
        sidecar.contains("extern \"C\" void sce_irq_restore(irq_state_t p0);"),
        "expected sce_irq_restore proto in cpp sidecar:\n{sidecar}",
    );
    // `#pragma once` per cpp sidecar template; not the #ifndef guard
    // form used by the C11 sidecar.
    assert!(sidecar.contains("#pragma once"));
}

#[test]
fn no_externs_no_sidecar() {
    // Atomic A's design lets a forge document have zero
    // `<sce:extern>` declarations — the sidecar must not emit in
    // that case (avoids dead-file pollution).
    let scxml = fixture_transform_with_externs("");
    let output = compile(&scxml, Language::Rust).expect("must compile without externs");
    assert!(
        find_sidecar(&output, "_externs.rs").is_none(),
        "expected no rust sidecar when extern_decls is empty",
    );
}

#[test]
fn rust_sidecar_carries_fence_with_void_return() {
    // Fence symbols have `()` sig — empty params, no return. Rust
    // emit must elide the `-> R` clause cleanly.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_fence_acquire" sig="()" abi="c"/>"##,
    );
    let output = compile(&scxml, Language::Rust).expect("must compile fence");
    let sidecar = find_sidecar(&output, "_externs.rs").expect("rust sidecar emitted");
    assert!(
        sidecar.contains("pub fn sce_atomic_fence_acquire();"),
        "expected fence decl with no return clause:\n{sidecar}",
    );
    // No stray `-> ()` artifact.
    assert!(
        !sidecar.contains("-> ()"),
        "fence emit must not carry `-> ()`:\n{sidecar}",
    );
}

#[test]
fn c11_fence_uses_void_for_empty_params() {
    // C11 strict-prototype convention: empty params lower to `(void)`.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_fence_acquire" sig="()" abi="c"/>"##,
    );
    let output = compile(&scxml, Language::C11).expect("must compile fence");
    let sidecar = find_sidecar(&output, "_externs.h").expect("c11 sidecar emitted");
    assert!(
        sidecar.contains("extern void sce_atomic_fence_acquire(void);"),
        "expected fence with `(void)` per C11 strict-prototype:\n{sidecar}",
    );
}

#[test]
fn extern_on_kotlin_rejected_via_mcu_class_family() {
    // Q-Call-7 lock: Kotlin/Go/Python reject `<sce:extern>` via the
    // `codegen/mcu-class-kind-on-non-mcu-language` family. The
    // existing diagnostic carries `kind = "<sce:extern>"` to
    // disambiguate from the kind-axis rejection on the same code.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match compile(&scxml, Language::Kotlin) {
        Ok(_) => panic!("kotlin must reject extern"),
        Err(e) => e,
    };
    match err {
        ForgeError::Generate(GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
            kind,
            language,
        }) => {
            assert_eq!(kind, "<sce:extern>");
            assert_eq!(language, "kotlin");
        }
        other => panic!("expected CodegenMcuClassKindOnNonMcuLanguage, got {other:?}"),
    }
}

#[test]
fn extern_on_go_rejected_via_mcu_class_family() {
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match compile(&scxml, Language::Go) {
        Ok(_) => panic!("go must reject extern"),
        Err(e) => e,
    };
    assert!(matches!(
        err,
        ForgeError::Generate(GenerateError::CodegenMcuClassKindOnNonMcuLanguage { language, .. })
            if language == "go"
    ));
}

#[test]
fn extern_on_python_rejected_via_mcu_class_family() {
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = match compile(&scxml, Language::Python) {
        Ok(_) => panic!("python must reject extern"),
        Err(e) => e,
    };
    assert!(matches!(
        err,
        ForgeError::Generate(GenerateError::CodegenMcuClassKindOnNonMcuLanguage { language, .. })
            if language == "python"
    ));
}

#[test]
fn no_externs_kotlin_compiles_unchanged() {
    // Atomic A semantics preserved on non-MCU when extern_decls is
    // empty — the rejection gate fires only when the document
    // actually carries `<sce:extern>`.
    let scxml = fixture_transform_with_externs("");
    let result = compile(&scxml, Language::Kotlin);
    if let Err(e) = &result {
        panic!("kotlin without externs must compile: {e:?}");
    }
}

#[test]
fn rust_sidecar_includes_atomic_cas_strong() {
    // CAS sigs have 3 params + return — exercises the param
    // numbering (`p0, p1, p2`).
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_cas_strong_acq_rel_u32" sig="(*mut u32, u32, u32) -> u32" abi="c"/>"##,
    );
    let output = compile(&scxml, Language::Rust).expect("must compile CAS");
    let sidecar = find_sidecar(&output, "_externs.rs").expect("rust sidecar emitted");
    assert!(
        sidecar.contains(
            "pub fn sce_atomic_cas_strong_acq_rel_u32(p0: *mut u32, p1: u32, p2: u32) -> u32;"
        ),
        "expected CAS decl in sidecar:\n{sidecar}",
    );
}
