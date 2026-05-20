// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// watching-zenoh RFC §5.I `<sce:extern>` whitelisted intrinsic registry
// — Atomic A end-to-end fixtures. Each test exercises
// `parse_forge_with_imports` against an inline SCXML carrying one or
// more `<sce:extern>` declarations and asserts the parse result:
//
//   - 4 reject fixtures, one per spec rejection axis
//     (`extern/symbol-not-in-whitelist`, `extern/abi-mismatch`,
//     `extern/signature-mismatch`, `extern/ordering-unspecified`)
//   - 1 happy fixture proving registry-clean declarations roundtrip
//     into `ParsedForge.externs`
//
// The fixtures wrap each `<sce:extern>` in a minimal `transform` kind
// (precedent: `tests/forge/resources/transform_*.scxml`) — the kind
// choice is incidental; `<sce:extern>` parsing happens at the root
// level alongside `<sce:import>`, regardless of `sce:kind`.

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::DocumentLabel;

/// Build a minimal `transform` kind SCXML wrapper around the supplied
/// `<sce:extern>` declarations. The transform's datamodel is empty
/// (no `sce:direction="out"` derivations) — a shape the kind parser
/// accepts since per-field validators don't see absent rows as a
/// failure mode in v1.
fn fixture_transform_with_externs(extern_decls: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="extern_test">
  {extern_decls}
  <datamodel>
    <data id="x" sce:type="uint32" sce:direction="in"/>
    <data id="y" sce:type="uint32" sce:direction="out" expr="x"/>
  </datamodel>
</scxml>
"##
    )
}

/// Convenience: invoke `parse_forge_with_imports` on a fixture string.
fn parse_fixture(scxml: &str) -> Result<sce_build::forge::model::ParsedForge, ForgeError> {
    parse_forge_with_imports(scxml, DocumentLabel::symmetric("extern_test"))
        .map(|opt| opt.expect("non-statechart kind"))
        .map_err(|e| e.error)
}

#[test]
fn happy_path_atomic_load_acquire_u32_roundtrips() {
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let parsed = parse_fixture(&scxml).expect("registry-clean declaration");
    assert_eq!(parsed.externs.len(), 1);
    let decl = &parsed.externs[0];
    assert_eq!(decl.name, "sce_atomic_load_acquire_u32");
    assert_eq!(decl.sig, "(*const u32) -> u32");
    assert_eq!(decl.abi, "c");
    // `crate` attribute omitted ⇒ falls back to registry's canonical
    // `sce_intrinsics_runtime`.
    assert_eq!(decl.crate_name, "sce_intrinsics_runtime");
    assert!(decl.line.is_some(), "line was not captured");
}

#[test]
fn happy_path_multiple_externs_preserve_order() {
    // C5 (spec §5.E line 1548): the cache-maintenance trio is
    // FSM-driven and rejected at parse time when authored via
    // `<sce:extern>`. Substitute `sce_atomic_fence_seq_cst` (a
    // non-cache fence symbol) so the order-preservation contract
    // stays exercised on three baseline-clean entries.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>
  <sce:extern name="sce_atomic_fence_seq_cst" sig="()" abi="c"/>
  <sce:extern name="sce_irq_save" sig="() -> irq_state_t" abi="c" crate="custom_crate"/>"##,
    );
    let parsed = parse_fixture(&scxml).expect("3 registry-clean declarations");
    assert_eq!(parsed.externs.len(), 3);
    assert_eq!(
        parsed.externs[0].name,
        "sce_atomic_load_acquire_u32"
    );
    assert_eq!(
        parsed.externs[1].name,
        "sce_atomic_fence_seq_cst"
    );
    assert_eq!(parsed.externs[2].name, "sce_irq_save");
    // Explicit `crate` attribute overrides the registry's canonical
    // crate (atomic-A storage of plugin-extension future axis).
    assert_eq!(parsed.externs[2].crate_name, "custom_crate");
}

#[test]
fn reject_symbol_not_in_whitelist() {
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_does_not_exist_in_registry" sig="()" abi="c"/>"##,
    );
    let err = parse_fixture(&scxml).expect_err("must reject unknown symbol");
    match err {
        ForgeError::Validation(ValidationError::ExternSymbolNotInWhitelist { name, .. }) => {
            assert_eq!(name, "sce_does_not_exist_in_registry");
        }
        other => panic!("expected ExternSymbolNotInWhitelist, got {other:?}"),
    }
}

#[test]
fn reject_abi_mismatch() {
    // `sce_atomic_load_acquire_u32` is registered with `abi="c"`; the
    // fixture writes `abi="rust"`.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="rust"/>"##,
    );
    let err = parse_fixture(&scxml).expect_err("must reject mismatched ABI");
    match err {
        ForgeError::Validation(ValidationError::ExternAbiMismatch {
            name,
            expected,
            actual,
        }) => {
            assert_eq!(name, "sce_atomic_load_acquire_u32");
            assert_eq!(expected, "c");
            assert_eq!(actual, "rust");
        }
        other => panic!("expected ExternAbiMismatch, got {other:?}"),
    }
}

#[test]
fn reject_signature_mismatch() {
    // Same name + abi as the registry; sig has the wrong return width.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u64" abi="c"/>"##,
    );
    let err = parse_fixture(&scxml).expect_err("must reject mismatched signature");
    match err {
        ForgeError::Validation(ValidationError::ExternSignatureMismatch {
            name,
            expected,
            actual,
        }) => {
            assert_eq!(name, "sce_atomic_load_acquire_u32");
            assert_eq!(expected, "(*const u32) -> u32");
            assert_eq!(actual, "(*const u32) -> u64");
        }
        other => panic!("expected ExternSignatureMismatch, got {other:?}"),
    }
}

#[test]
fn reject_ordering_unspecified() {
    // Atomic-family base name without `_<ordering>_<width>` suffix.
    // `sce_atomic_load` is a known base; writing it without the
    // suffix triggers the spec-line-1850 specialized rejection rather
    // than the generic `extern/symbol-not-in-whitelist`.
    let scxml = fixture_transform_with_externs(
        r##"<sce:extern name="sce_atomic_load" sig="(*const u32) -> u32" abi="c"/>"##,
    );
    let err = parse_fixture(&scxml).expect_err("must reject suffix-less atomic base");
    match err {
        ForgeError::Validation(ValidationError::ExternOrderingUnspecified {
            base,
            candidates,
            ..
        }) => {
            assert_eq!(base, "sce_atomic_load");
            // 2 orderings (acquire, relaxed) × 5 widths = 10 completions.
            assert_eq!(candidates.len(), 10);
            assert!(candidates.contains(&"sce_atomic_load_acquire_u32".to_string()));
        }
        other => panic!("expected ExternOrderingUnspecified, got {other:?}"),
    }
}

#[test]
fn empty_externs_when_none_authored() {
    // A document with zero `<sce:extern>` children must yield an empty
    // list — confirms the parser hook does not synthesize entries.
    let scxml = fixture_transform_with_externs("");
    let parsed = parse_fixture(&scxml).expect("happy path with no externs");
    assert!(parsed.externs.is_empty());
}
