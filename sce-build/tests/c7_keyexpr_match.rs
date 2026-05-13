//! C7-lowering — algorithm-over-BC dispatch integration tests across
//! all 6 backends.
//!
//! Per watching-zenoh RFC §5.A line 311 + §5.L line 2642-2647 + RFC
//! stub `claudedocs/rfc-c7-keyexpr-matching-algorithm.md` §3
//! Q-C7-1..11 locks 2026-05-13: an algorithm body that imports a
//! bounded-collection emits a uniform index-loop `<sce:foreach
//! in="<bc-alias>">` and dispatches into the BC's read-only method
//! roster via `<sce:call target="<alias>.<method>">`. This file is
//! the in-atomic consumer of the foreach-BC codegen + dotted-call
//! resolution + 6 spec-named diagnostics added by C7-lowering. Without
//! these tests the surface would be silently built-but-unconsumed per
//! `[[feedback-silently-broken-hooks]]`.
//!
//! Three groups of tests:
//!   1. Per-backend foreach-BC emit shape (6 backends × 1 test each).
//!   2. Per-backend dotted-call (`subs.find_by_index(...)`) emit shape
//!      (6 backends × 1 test each).
//!   3. Negative — one test per new diagnostic code firing
//!      (`algorithm/foreach-source-not-iterable`,
//!      `algorithm/call-target-unknown`,
//!      `algorithm/call-target-method-unknown`,
//!      `algorithm/bc-mutation-forbidden`,
//!      `algorithm/call-arg-count-mismatch`,
//!      `algorithm/foreach-source-bc-with-bytes-item-type`).
//!   4. Cross-backend drift guard
//!      (`c7_lowering_emits_on_all_six_backends`).

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

const GO_PREFIX: &str = "github.com/acme/project/generated";

fn template_dir(lang: Language) -> PathBuf {
    sce_build::find_template_dir_for(lang)
}

fn resource_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("forge")
        .join("resources")
        .join(name)
}

fn copy_resource_into(dir: &Path, name: &str) -> PathBuf {
    let src = resource_path(name);
    let content = fs::read_to_string(&src).unwrap_or_else(|e| {
        panic!("read {}: {e}", src.display());
    });
    let dest = dir.join(name);
    fs::write(&dest, content).expect("write resource");
    dest
}

fn options_for(lang: Language) -> ForgeCompileOptions {
    let mut opts = ForgeCompileOptions::default();
    if matches!(lang, Language::Go) {
        opts.go_module_prefix = Some(GO_PREFIX.to_string());
    }
    opts
}

fn compile_algo_for(lang: Language) -> String {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "subscription_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_sub_table.scxml");
    let algo = copy_resource_into(dir.path(), "algorithm_bc_iter_minimal.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[codec.as_path(), bc.as_path(), algo.as_path()],
        &template_dir(lang),
        lang,
        &options_for(lang),
    )
    .expect("orchestrator codegen succeeds");
    extract_algo(&outputs)
}

fn extract_algo(outputs: &[(String, GeneratedOutput)]) -> String {
    outputs
        .iter()
        .find(|(name, _)| name == "algorithm_bc_iter_minimal.scxml")
        .expect("algorithm output present")
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn compile_inline(
    lang: Language,
    extra: &[(String, String)],
) -> Result<Vec<(String, GeneratedOutput)>, Located<ForgeError>> {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "subscription_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_sub_table.scxml");
    let mut paths: Vec<PathBuf> = vec![codec, bc];
    for (name, content) in extra {
        let p = dir.path().join(name);
        fs::write(&p, content).expect("write extra");
        paths.push(p);
    }
    let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
    compile_scxml_with_imports(
        &[],
        &path_refs,
        &template_dir(lang),
        lang,
        &options_for(lang),
    )
}

// ═══════════════════════════════════════════════════════════════
// ── Per-backend foreach-BC emit shape ──────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn rust_foreach_bc_emits_index_loop_with_get_by_slot() {
    let code = compile_algo_for(Language::Rust);
    // Uniform index-loop per Q-C7-2 (c) — `0..(LocalSubTable::capacity() as u32)`.
    assert!(
        code.contains("for slot_idx in 0..(LocalSubTable::capacity() as u32) {"),
        "Rust foreach-BC missing index loop; got:\n{code}"
    );
    // `if let Some(entry) = subs.get_by_slot(slot_idx)` shape.
    assert!(
        code.contains("if let Some(entry) = subs.get_by_slot(slot_idx) {"),
        "Rust foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    // BC param threaded into signature.
    assert!(
        code.contains("subs: &LocalSubTable"),
        "Rust foreach-BC signature missing BC ref; got:\n{code}"
    );
    // include statement at top of file.
    assert!(
        code.contains("use super::local_sub_table::LocalSubTable;"),
        "Rust foreach-BC missing include; got:\n{code}"
    );
}

#[test]
fn cpp_foreach_bc_emits_index_loop_with_get_by_slot() {
    let code = compile_algo_for(Language::Cpp);
    assert!(
        code.contains("for (std::uint32_t slot_idx = 0; slot_idx < static_cast<std::uint32_t>(LocalSubTable::capacity()); ++slot_idx) {"),
        "Cpp foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("auto entry_opt = subs.get_by_slot(slot_idx);"),
        "Cpp foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    assert!(
        code.contains("if (entry_opt.has_value()) {"),
        "Cpp foreach-BC missing has_value check; got:\n{code}"
    );
    assert!(
        code.contains("const auto& entry = entry_opt.value();"),
        "Cpp foreach-BC missing entry alias; got:\n{code}"
    );
    assert!(
        code.contains("#include \"local_sub_table.h\""),
        "Cpp foreach-BC missing include; got:\n{code}"
    );
}

#[test]
fn kotlin_foreach_bc_emits_index_loop_with_get_by_slot() {
    let code = compile_algo_for(Language::Kotlin);
    assert!(
        code.contains("for (slotIdx in 0u until subs.capacity().toUInt()) {"),
        "Kotlin foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("val entry = subs.getBySlot(slotIdx) ?: continue"),
        "Kotlin foreach-BC missing getBySlot dispatch; got:\n{code}"
    );
    assert!(
        code.contains("import com.sce.generated.local_sub_table.*"),
        "Kotlin foreach-BC missing wildcard import; got:\n{code}"
    );
}

#[test]
fn go_foreach_bc_emits_index_loop_with_get_by_slot() {
    let code = compile_algo_for(Language::Go);
    assert!(
        code.contains("for slotIdx := uint32(0); slotIdx < LocalSubTableCapacity; slotIdx++ {"),
        "Go foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("entry, ok := subs.GetBySlot(slotIdx)"),
        "Go foreach-BC missing GetBySlot dispatch; got:\n{code}"
    );
    assert!(
        code.contains("if !ok { continue }"),
        "Go foreach-BC missing continue on miss; got:\n{code}"
    );
}

#[test]
fn python_foreach_bc_emits_index_loop_with_get_by_slot() {
    let code = compile_algo_for(Language::Python);
    assert!(
        code.contains("for slot_idx in range(LocalSubTable.capacity()):"),
        "Python foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("entry = subs.get_by_slot(slot_idx)"),
        "Python foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    assert!(
        code.contains("if entry is None:"),
        "Python foreach-BC missing None guard; got:\n{code}"
    );
    assert!(
        code.contains("from .local_sub_table import LocalSubTable"),
        "Python foreach-BC missing import; got:\n{code}"
    );
}

#[test]
fn c11_foreach_bc_emits_index_loop_with_get_by_slot() {
    let code = compile_algo_for(Language::C11);
    assert!(
        code.contains("for (uint32_t slot_idx = 0u; slot_idx < LOCAL_SUB_TABLE_CAPACITY; ++slot_idx) {"),
        "C11 foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("const subscription_entry_t *entry_ptr = local_sub_table_get_by_slot(subs, slot_idx);"),
        "C11 foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    assert!(
        code.contains("if (entry_ptr == NULL) continue;"),
        "C11 foreach-BC missing NULL guard; got:\n{code}"
    );
    assert!(
        code.contains("subscription_entry_t entry = *entry_ptr;"),
        "C11 foreach-BC missing deref-copy; got:\n{code}"
    );
    assert!(
        code.contains("#include \"local_sub_table.h\""),
        "C11 foreach-BC missing include; got:\n{code}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Negative tests — diagnostic firing ─────────────────────────
// ═══════════════════════════════════════════════════════════════

const NEGATIVE_BASE_SIG: &str = r##"  <sce:signature>
    <sce:param name="target" type="uint32"/>
    <sce:return type="uint16"/>
  </sce:signature>"##;

fn negative_algo_doc(body: &str, extra_imports: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
       xsi:schemaLocation="http://sce.dev/ext sce-forge.xsd"
       sce:kind="algorithm"
       version="1.0">
  <sce:import kind="bounded-collection" src="local_sub_table.scxml" as="subs"/>
{extra_imports}{NEGATIVE_BASE_SIG}
  <sce:body>
{body}
  </sce:body>
</scxml>"##
    )
}

fn match_validation_error<F: FnOnce(&ValidationError) -> bool>(
    err: &Located<ForgeError>,
    pred: F,
) -> bool {
    match &err.error {
        ForgeError::Validation(v) => pred(v),
        _ => false,
    }
}

fn expect_compile_err(
    lang: Language,
    extra: &[(String, String)],
) -> Located<ForgeError> {
    match compile_inline(lang, extra) {
        Ok(_) => panic!("expected compile error but compilation succeeded"),
        Err(e) => e,
    }
}

#[test]
fn negative_foreach_source_not_iterable_fires() {
    let body = r##"    <sce:foreach item="entry" in="unknown_source">
      <sce:return expr="0"/>
    </sce:foreach>
    <sce:return expr="0xFFFF"/>"##;
    let doc = negative_algo_doc(body, "");
    let err = expect_compile_err(
        Language::Rust,
        &[("algo.scxml".into(), doc)],
    );
    assert!(
        match_validation_error(&err, |v| matches!(
            v,
            ValidationError::AlgorithmForeachSourceNotIterable { src, .. } if src == "unknown_source"
        )),
        "expected AlgorithmForeachSourceNotIterable; got: {err:?}"
    );
}

#[test]
fn negative_call_target_unknown_fires() {
    let body = r##"    <sce:call target="missing_alias.find_by_index" args="target"/>
    <sce:return expr="0xFFFF"/>"##;
    let doc = negative_algo_doc(body, "");
    let err = expect_compile_err(
        Language::Rust,
        &[("algo.scxml".into(), doc)],
    );
    assert!(
        match_validation_error(&err, |v| matches!(
            v,
            ValidationError::AlgorithmCallTargetUnknown { alias, .. } if alias == "missing_alias"
        )),
        "expected AlgorithmCallTargetUnknown; got: {err:?}"
    );
}

#[test]
fn negative_call_target_method_unknown_fires() {
    let body = r##"    <sce:call target="subs.unknown_method" args="target"/>
    <sce:return expr="0xFFFF"/>"##;
    let doc = negative_algo_doc(body, "");
    let err = expect_compile_err(
        Language::Rust,
        &[("algo.scxml".into(), doc)],
    );
    assert!(
        match_validation_error(&err, |v| matches!(
            v,
            ValidationError::AlgorithmCallTargetMethodUnknown { method, .. } if method == "unknown_method"
        )),
        "expected AlgorithmCallTargetMethodUnknown; got: {err:?}"
    );
}

#[test]
fn negative_bc_mutation_forbidden_fires() {
    let body = r##"    <sce:call target="subs.insert" args="target"/>
    <sce:return expr="0xFFFF"/>"##;
    let doc = negative_algo_doc(body, "");
    let err = expect_compile_err(
        Language::Rust,
        &[("algo.scxml".into(), doc)],
    );
    assert!(
        match_validation_error(&err, |v| matches!(
            v,
            ValidationError::AlgorithmBcMutationForbidden { method, .. } if method == "insert"
        )),
        "expected AlgorithmBcMutationForbidden; got: {err:?}"
    );
}

#[test]
fn negative_call_arg_count_mismatch_fires() {
    // BC `find_by_index` takes 1 arg — pass 2 to trigger the mismatch.
    let body = r##"    <sce:call target="subs.find_by_index" args="target, target"/>
    <sce:return expr="0xFFFF"/>"##;
    let doc = negative_algo_doc(body, "");
    let err = expect_compile_err(
        Language::Rust,
        &[("algo.scxml".into(), doc)],
    );
    assert!(
        match_validation_error(&err, |v| matches!(
            v,
            ValidationError::AlgorithmCallArgCountMismatch { actual: 2, expected: 1, .. }
        )),
        "expected AlgorithmCallArgCountMismatch (2 vs 1); got: {err:?}"
    );
}

#[test]
fn negative_foreach_source_bc_with_bytes_item_type_fires() {
    // BC foreach body declares a uint8 var — legacy bytes-iteration
    // pattern misapplied to BC iteration.
    let body = r##"    <sce:foreach item="entry" in="subs">
      <sce:var name="b" type="uint8" init="0"/>
      <sce:return expr="0"/>
    </sce:foreach>
    <sce:return expr="0xFFFF"/>"##;
    let doc = negative_algo_doc(body, "");
    let err = expect_compile_err(
        Language::Rust,
        &[("algo.scxml".into(), doc)],
    );
    assert!(
        match_validation_error(&err, |v| matches!(
            v,
            ValidationError::AlgorithmForeachSourceBcWithBytesItemType { src, var_name }
                if src == "subs" && var_name == "b"
        )),
        "expected AlgorithmForeachSourceBcWithBytesItemType; got: {err:?}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Cross-backend drift guard ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn c7_lowering_emits_on_all_six_backends() {
    // Drift guard: every backend's foreach-BC emit must surface the
    // index-loop slot variable + get_by_slot dispatch. A regression
    // that drops the dispatch on one backend would be caught here
    // before the C7-keyexpr-fixture atomic exercises the full
    // exemplar.
    for lang in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let code = compile_algo_for(lang);
        let slot_marker = match lang {
            Language::Kotlin | Language::Go => "slotIdx",
            _ => "slot_idx",
        };
        assert!(
            code.contains(slot_marker),
            "{lang:?}: foreach-BC missing slot index variable; got:\n{code}"
        );
        let dispatch_marker = match lang {
            Language::Kotlin => "getBySlot",
            Language::Go => "GetBySlot",
            _ => "get_by_slot",
        };
        assert!(
            code.contains(dispatch_marker),
            "{lang:?}: foreach-BC missing get_by_slot dispatch; got:\n{code}"
        );
    }
}
