//! C7 keyexpr-fixture — reference exemplar parity test across all 6
//! backends.
//!
//! Per watching-zenoh RFC §5.A line 311 + §5.L line 2642-2647 + RFC
//! stub `claudedocs/rfc-c7-keyexpr-matching-algorithm.md` §3 Q-C7-8
//! (a) + Q-C7-9 (a) + Q-C7-11 (c) locks 2026-05-13: the C7 chain's
//! sub-atomic 3 of 3. Ships two fixtures in `tests/forge/resources/`:
//!
//! - `algorithm_keyexpr_intersect_exact.scxml` — inner algorithm,
//!   `(entry_id: u32, target_id: u32) -> bool` exact equality.
//! - `algorithm_keyexpr_match_first.scxml` — outer algorithm, imports
//!   BC + inner algorithm, iterates BC via `<sce:foreach in="subs">`,
//!   dispatches `km(entry.callback_id, target)` via expression-form
//!   qualified-call rename. Returns first matching slot index.
//!
//! Drives §6.2.6 cross-language byte-equivalence parity test
//! extension — every backend's emit must surface both the foreach-BC
//! index-loop (already validated in `c7_keyexpr_match.rs` against
//! the minimal exemplar) AND the cross-algorithm dispatch substituted
//! by `validate_and_enrich_imports::build_qualified_call`. Runtime
//! parity (compile + run + diff outputs) defers per RFC §A6 to a
//! future atomic when the cross-language test-vector harness lands.
//!
//! In-atomic consumer = the per-backend emit assertions below.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
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

fn compile_match_first_for(lang: Language) -> String {
    let dir = tempdir().expect("tempdir");
    let codec = copy_resource_into(dir.path(), "subscription_entry.scxml");
    let bc = copy_resource_into(dir.path(), "local_sub_table.scxml");
    let inner = copy_resource_into(dir.path(), "algorithm_keyexpr_intersect_exact.scxml");
    let outer = copy_resource_into(dir.path(), "algorithm_keyexpr_match_first.scxml");
    let outputs = compile_scxml_with_imports(
        &[],
        &[
            codec.as_path(),
            bc.as_path(),
            inner.as_path(),
            outer.as_path(),
        ],
        &template_dir(lang),
        lang,
        &options_for(lang),
        None,
    )
    .expect("orchestrator codegen succeeds");
    extract_match_first(&outputs)
}

fn extract_match_first(outputs: &[(String, GeneratedOutput)]) -> String {
    outputs
        .iter()
        .find(|(name, _)| name == "algorithm_keyexpr_match_first.scxml")
        .expect("match_first output present")
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ═══════════════════════════════════════════════════════════════
// ── Per-backend keyexpr_match_first emit shape ─────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn rust_keyexpr_match_first_emits_foreach_bc_and_cross_algo_dispatch() {
    let code = compile_match_first_for(Language::Rust);
    // foreach-BC index-loop (validated in c7_keyexpr_match.rs against
    // the minimal exemplar — this fixture exercises the same surface
    // under cross-algo dispatch).
    assert!(
        code.contains("for slot_idx in 0..(LocalSubTable::capacity() as u32) {"),
        "Rust foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("if let Some(entry) = subs.get_by_slot(slot_idx) {"),
        "Rust foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    // Cross-algorithm dispatch — `km(entry.callback_id, target)`
    // alias-rename emits the imported algorithm's qualified call
    // (`<namespace>::<func>` for Rust). Identity SSOT: the module is named
    // from the algorithm's `name=` attribute (`keyexpr_intersect`), not its
    // file stem (`algorithm_keyexpr_intersect_exact`).
    assert!(
        code.contains("keyexpr_intersect::keyexpr_intersect(entry.callback_id, target)"),
        "Rust cross-algo dispatch missing qualified call; got:\n{code}"
    );
    // Both BC and algorithm imports surface as `use super::*` lines.
    assert!(
        code.contains("use super::local_sub_table::LocalSubTable;"),
        "Rust BC import missing; got:\n{code}"
    );
    assert!(
        code.contains("use super::keyexpr_intersect;"),
        "Rust algorithm import missing; got:\n{code}"
    );
}

#[test]
fn cpp_keyexpr_match_first_emits_foreach_bc_and_cross_algo_dispatch() {
    let code = compile_match_first_for(Language::Cpp);
    assert!(
        code.contains("for (std::uint32_t slot_idx = 0; slot_idx < static_cast<std::uint32_t>(::SCE::Generated::LocalSubTable::LocalSubTable::capacity()); ++slot_idx) {"),
        "Cpp foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("auto entry_opt = subs.get_by_slot(slot_idx);"),
        "Cpp foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    // Cross-algorithm dispatch — Cpp qualified call form. Identity SSOT:
    // the namespace is named from the algorithm's `name=` attribute
    // (`SCE::Generated::KeyexprIntersect`), not its file stem.
    assert!(
        code.contains("KeyexprIntersect::keyexpr_intersect(entry.callback_id, target)"),
        "Cpp cross-algo dispatch missing qualified call; got:\n{code}"
    );
    assert!(
        code.contains("#include \"local_sub_table.h\""),
        "Cpp BC import missing; got:\n{code}"
    );
    // The algorithm header is named by its `name=` attribute
    // (`keyexpr_intersect.h`), not the import's file stem.
    assert!(
        code.contains("#include \"keyexpr_intersect.h\""),
        "Cpp algorithm import missing; got:\n{code}"
    );
}

#[test]
fn kotlin_keyexpr_match_first_emits_foreach_bc_and_cross_algo_dispatch() {
    let code = compile_match_first_for(Language::Kotlin);
    assert!(
        code.contains("for (slotIdx in 0u until subs.capacity().toUInt()) {"),
        "Kotlin foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("val entry = subs.getBySlot(slotIdx) ?: continue"),
        "Kotlin foreach-BC missing getBySlot dispatch; got:\n{code}"
    );
    // Cross-algorithm dispatch — Kotlin wildcard import brings the
    // top-level `fun keyexprIntersect(...)` into scope, so the bare
    // camelCase form resolves without a qualifier.
    assert!(
        code.contains("keyexprIntersect(entry.callback_id, target)"),
        "Kotlin cross-algo dispatch missing bare call; got:\n{code}"
    );
    assert!(
        code.contains("import com.sce.generated.local_sub_table.*"),
        "Kotlin BC wildcard import missing; got:\n{code}"
    );
    // Identity SSOT: the package is named from the algorithm's `name=`
    // attribute (`keyexpr_intersect`), not its file stem.
    assert!(
        code.contains("import com.sce.generated.keyexpr_intersect.*"),
        "Kotlin algorithm wildcard import missing; got:\n{code}"
    );
}

#[test]
fn go_keyexpr_match_first_emits_foreach_bc_and_cross_algo_dispatch() {
    let code = compile_match_first_for(Language::Go);
    assert!(
        code.contains(
            "for slotIdx := uint32(0); slotIdx < local_sub_table.LocalSubTableCapacity; slotIdx++ {"
        ),
        "Go foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("entry, ok := subs.GetBySlot(slotIdx)"),
        "Go foreach-BC missing GetBySlot dispatch; got:\n{code}"
    );
    // Cross-algorithm dispatch — Go package-qualified form. Identity SSOT:
    // the package is named from the algorithm's `name=` attribute
    // (`keyexpr_intersect`), not its file stem. The element field read is
    // exported PascalCase (`entry.CallbackId`) to bind against the Go codec
    // struct field (`codec_field_id` SSOT).
    assert!(
        code.contains("keyexpr_intersect.KeyexprIntersect(entry.CallbackId, target)"),
        "Go cross-algo dispatch missing qualified call; got:\n{code}"
    );
}

#[test]
fn python_keyexpr_match_first_emits_foreach_bc_and_cross_algo_dispatch() {
    let code = compile_match_first_for(Language::Python);
    assert!(
        code.contains("for slot_idx in range(LocalSubTable.capacity()):"),
        "Python foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains("entry = subs.get_by_slot(slot_idx)"),
        "Python foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    // Cross-algorithm dispatch — Python module-qualified form
    // (`from . import <snake>` + `<snake>.<func>(...)`). Identity SSOT: the
    // module is named from the algorithm's `name=` attribute
    // (`keyexpr_intersect`), not its file stem.
    assert!(
        code.contains("keyexpr_intersect.keyexpr_intersect(entry.callback_id, target)"),
        "Python cross-algo dispatch missing qualified call; got:\n{code}"
    );
    assert!(
        code.contains("from .local_sub_table import LocalSubTable"),
        "Python BC import missing; got:\n{code}"
    );
    assert!(
        code.contains("from . import keyexpr_intersect"),
        "Python algorithm import missing; got:\n{code}"
    );
}

#[test]
fn c11_keyexpr_match_first_emits_foreach_bc_and_cross_algo_dispatch() {
    let code = compile_match_first_for(Language::C11);
    assert!(
        code.contains(
            "for (uint32_t slot_idx = 0u; slot_idx < LOCAL_SUB_TABLE_CAPACITY; ++slot_idx) {"
        ),
        "C11 foreach-BC missing index loop; got:\n{code}"
    );
    assert!(
        code.contains(
            "const subscription_entry_t *entry_ptr = local_sub_table_get_by_slot(subs, slot_idx);"
        ),
        "C11 foreach-BC missing get_by_slot dispatch; got:\n{code}"
    );
    // Cross-algorithm dispatch — C7 §A6: the algorithm kind names its
    // emitted symbol by its `name=` attribute, and C11 has no namespace,
    // so the cross-doc call is the bare canonical symbol (resolving
    // against the bare `static inline` definition), not a
    // `<file_stem>_<func>` prefix.
    assert!(
        code.contains("keyexpr_intersect(entry.callback_id, target)"),
        "C11 cross-algo dispatch missing bare call; got:\n{code}"
    );
    assert!(
        code.contains("#include \"local_sub_table.h\""),
        "C11 BC import missing; got:\n{code}"
    );
    // The algorithm header is named by its `name=` attribute
    // (`keyexpr_intersect.h`), so the include must reference that file —
    // not the import's file stem (which is never written as a header).
    assert!(
        code.contains("#include \"keyexpr_intersect.h\""),
        "C11 algorithm import missing; got:\n{code}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── §6.2.6 cross-backend parity drift guard ────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn keyexpr_match_first_emits_on_all_six_backends() {
    // Drift guard: every backend's keyexpr_match_first emit must
    // surface both axes (BC iter + cross-algo dispatch). Regression
    // on either axis on any backend would be caught here before the
    // watching-zenoh-side C8 / C12 fixtures (which author their own
    // keyexpr_intersect variants against this v1 contract) exercise
    // the full pattern.
    for lang in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let code = compile_match_first_for(lang);
        // Axis 1: BC iteration surfaces.
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
        // Axis 2: cross-algorithm dispatch surfaces. Kotlin's wildcard
        // import and C11's no-namespace model both lift the bare symbol
        // into scope (C7 §A6); every other backend uses a qualified
        // prefix matching its `build_qualified_call` shape (Rust/Cpp
        // `::`, Go/Python `.`).
        // Identity SSOT: every qualifier is named from the algorithm's
        // `name=` attribute (`keyexpr_intersect` / `KeyexprIntersect`), not
        // its file stem (`algorithm_keyexpr_intersect_exact`).
        let cross_call_marker = match lang {
            Language::Kotlin => "keyexprIntersect(",
            Language::Go => "keyexpr_intersect.KeyexprIntersect(",
            Language::C11 => "keyexpr_intersect(entry.callback_id, target)",
            Language::Cpp => "KeyexprIntersect::keyexpr_intersect(",
            Language::Python => "keyexpr_intersect.keyexpr_intersect(",
            Language::Rust => "keyexpr_intersect::keyexpr_intersect(",
        };
        assert!(
            code.contains(cross_call_marker),
            "{lang:?}: cross-algo dispatch missing `{cross_call_marker}`; got:\n{code}"
        );
    }
}
