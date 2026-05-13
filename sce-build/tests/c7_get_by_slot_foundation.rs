//! C7-foundation — `get_by_slot` method emit integration tests
//! across all 6 BC backends.
//!
//! Per watching-zenoh RFC §5.L line 2642-2647 + RFC stub
//! `claudedocs/rfc-c7-keyexpr-matching-algorithm.md` §3 Q-C7-2 (c)
//! lock 2026-05-13: BC iteration from algorithm bodies lowers to an
//! index loop using `len()` + a new slot-indexed read `get_by_slot
//! (slot) -> Option<&T>` (per-backend idiom). This file is the
//! in-atomic consumer of the new method — without these tests the
//! 6 template additions would be silently built-but-unconsumed per
//! `[[feedback-silently-broken-hooks]]`. C7-lowering (atomic 2 of
//! 3) will consume `get_by_slot` from the foreach-BC codegen path.
//!
//! Test strategy mirrors γ2/γ3/γ4: emit-shape grep against the
//! per-backend signature + boundary-check body. Per-backend method
//! shape (mirroring existing `get(handle)`):
//! - Rust:   `pub fn get_by_slot(&self, slot: u32) -> Option<&T>`
//! - Cpp:    `std::optional<TType> get_by_slot(std::uint32_t slot) const`
//! - Kotlin: `fun getBySlot(slot: UInt): T?`
//! - Go:     `func (t *BC) GetBySlot(slot uint32) (*T, bool)`
//! - Python: `def get_by_slot(self, slot: int) -> Optional[T]:`
//! - C11:    `static inline const T *<snake>_get_by_slot(const <snake>_t *self, uint32_t slot)`
//!
//! Semantic invariant (uniform across all 6 backends):
//! - OOB slot (>= CAPACITY) → returns the None-equivalent.
//! - Unoccupied slot → returns the None-equivalent.
//! - **No generation check** — slot-based access is for iteration
//!   paths; the use-after-remove guard lives on the Handle-based
//!   `get(handle)`.

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

fn write_doc(dir: &Path, basename: &str, content: &str) -> PathBuf {
    let path = dir.join(basename);
    fs::write(&path, content).expect("write doc");
    path
}

fn codec_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <datamodel>
    <sce:field id="key_expr_id" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>
    <sce:field id="callback_id" sce:type="uint32" sce:byte="4" sce:bit-size="32"/>
  </datamodel>
</scxml>"##
    )
}

fn bc_doc(name: &str, element_type: &str, capacity: u32) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="{name}" version="1.0">
  <sce:element-type>{element_type}</sce:element-type>
  <sce:capacity const="{capacity}"/>
</scxml>"##
    )
}

fn options_for(lang: Language) -> ForgeCompileOptions {
    let mut opts = ForgeCompileOptions::default();
    if matches!(lang, Language::Go) {
        opts.go_module_prefix = Some(GO_PREFIX.to_string());
    }
    opts
}

fn compile_bc_for(lang: Language) -> String {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8);
    let dir = tempdir().expect("tempdir");
    let codec_path = write_doc(dir.path(), "subscription_entry.scxml", &codec);
    let bc_path = write_doc(dir.path(), "local_sub_table.scxml", &bc);
    let outputs = compile_scxml_with_imports(
        &[],
        &[codec_path.as_path(), bc_path.as_path()],
        &template_dir(lang),
        lang,
        &options_for(lang),
    )
    .expect("orchestrator codegen succeeds");
    extract_bc(&outputs)
}

fn extract_bc(outputs: &[(String, GeneratedOutput)]) -> String {
    outputs
        .iter()
        .find(|(name, _)| name == "local_sub_table.scxml")
        .expect("BC output present")
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ═══════════════════════════════════════════════════════════════
// ── Per-backend get_by_slot signature emit ─────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn rust_get_by_slot_signature_and_boundary_checks() {
    let code = compile_bc_for(Language::Rust);

    // Signature: pub fn get_by_slot(&self, slot: u32) -> Option<&T>.
    assert!(
        code.contains("pub fn get_by_slot(&self, slot: u32) -> Option<&SubscriptionEntry>"),
        "Rust signature missing; got:\n{code}"
    );
    // OOB check + unoccupied check (count = 2 — both branches emit).
    assert!(code.contains("if slot >= CAPACITY"));
    assert!(code.contains("if !self.bit_is_set(slot)"));
    // No generation check — slot-based access bypasses the
    // use-after-remove guard.
    let body_start = code.find("pub fn get_by_slot").expect("get_by_slot present");
    let body_end = code[body_start..]
        .find("\n    }\n")
        .map(|e| body_start + e)
        .unwrap_or_else(|| code.len());
    let body = &code[body_start..body_end];
    assert!(
        !body.contains("self.generation[slot]"),
        "Rust get_by_slot must NOT do generation check (Handle-based get's responsibility); body:\n{body}"
    );
}

#[test]
fn cpp_get_by_slot_signature_and_boundary_checks() {
    let code = compile_bc_for(Language::Cpp);

    // Signature: std::optional<SubscriptionEntryType> get_by_slot(std::uint32_t slot) const.
    assert!(
        code.contains(
            "std::optional<SubscriptionEntryType> get_by_slot(std::uint32_t slot) const"
        ),
        "Cpp signature missing; got:\n{code}"
    );
    // OOB check + unoccupied check.
    let body_start = code.find("get_by_slot(std::uint32_t").expect("get_by_slot present");
    let body_end = code[body_start..]
        .find("\n    }\n")
        .map(|e| body_start + e)
        .expect("get_by_slot body closes");
    let body = &code[body_start..body_end];
    assert!(body.contains("if (s >= CAPACITY) return std::nullopt"));
    assert!(body.contains("if (!bitmap_.test(s)) return std::nullopt"));
    assert!(
        !body.contains("generation_[s]"),
        "Cpp get_by_slot must NOT do generation check; body:\n{body}"
    );
}

#[test]
fn kotlin_get_by_slot_signature_and_boundary_checks() {
    let code = compile_bc_for(Language::Kotlin);

    // Signature: fun getBySlot(slot: UInt): SubscriptionEntry?.
    assert!(
        code.contains("fun getBySlot(slot: UInt): SubscriptionEntry?"),
        "Kotlin signature missing; got:\n{code}"
    );
    let body_start = code.find("fun getBySlot").expect("getBySlot present");
    let body_end = code[body_start..]
        .find("\n    }\n")
        .map(|e| body_start + e)
        .expect("getBySlot body closes");
    let body = &code[body_start..body_end];
    assert!(body.contains("if (slotIdx >= CAPACITY) return null"));
    assert!(body.contains("if (!occupied[slotIdx]) return null"));
    assert!(
        !body.contains("generation[slotIdx]"),
        "Kotlin getBySlot must NOT do generation check; body:\n{body}"
    );
}

#[test]
fn go_get_by_slot_signature_and_boundary_checks() {
    let code = compile_bc_for(Language::Go);

    // Signature: func (t *LocalSubTable) GetBySlot(slot uint32) (*subscription_entry.SubscriptionEntry, bool).
    assert!(
        code.contains(
            "func (t *LocalSubTable) GetBySlot(slot uint32) (*subscription_entry.SubscriptionEntry, bool) {"
        ),
        "Go signature missing; got:\n{code}"
    );
    let body_start = code.find("func (t *LocalSubTable) GetBySlot").expect("GetBySlot present");
    let body_end = code[body_start..]
        .find("\n}\n")
        .map(|e| body_start + e)
        .expect("GetBySlot body closes");
    let body = &code[body_start..body_end];
    assert!(body.contains("if slot >= LocalSubTableCapacity {"));
    assert!(body.contains("if !t.occupied[slot] {"));
    assert!(
        !body.contains("t.generation[slot]"),
        "Go GetBySlot must NOT do generation check; body:\n{body}"
    );
}

#[test]
fn python_get_by_slot_signature_and_boundary_checks() {
    let code = compile_bc_for(Language::Python);

    // Signature: def get_by_slot(self, slot: int) -> Optional[SubscriptionEntry]:.
    assert!(
        code.contains("def get_by_slot(self, slot: int) -> Optional[SubscriptionEntry]:"),
        "Python signature missing; got:\n{code}"
    );
    let body_start = code.find("def get_by_slot").expect("get_by_slot present");
    let body_end = code[body_start..]
        .find("\n    def ")
        .map(|e| body_start + e)
        .unwrap_or_else(|| code.len());
    let body = &code[body_start..body_end];
    // OOB check accepts both `slot < 0` (defensive) and `slot >= CAPACITY`.
    assert!(body.contains("if slot < 0 or slot >= CAPACITY:"));
    assert!(body.contains("if not self._occupied[slot]:"));
    assert!(
        !body.contains("self._gen_get(slot)"),
        "Python get_by_slot must NOT do generation check; body:\n{body}"
    );
}

#[test]
fn c11_get_by_slot_signature_and_boundary_checks() {
    let code = compile_bc_for(Language::C11);

    // Signature: static inline const subscription_entry_t
    //                *local_sub_table_get_by_slot(
    //                    const local_sub_table_t *self, uint32_t slot)
    // The element-type reference matches the codec's emitted typedef
    // `<element_snake>_t` (codec template's `c_struct_typedef`), so the
    // BC's `#include "<element_snake>.h"` brings the type into scope
    // without requiring an additional PascalCase alias.
    assert!(
        code.contains(
            "static inline const subscription_entry_t *local_sub_table_get_by_slot(\n    const local_sub_table_t *self,\n    uint32_t slot)"
        ),
        "C11 signature missing; got:\n{code}"
    );
    let body_start = code.find("local_sub_table_get_by_slot(\n").expect("get_by_slot present");
    let body_end = code[body_start..]
        .find("\n}\n")
        .map(|e| body_start + e)
        .expect("get_by_slot body closes");
    let body = &code[body_start..body_end];
    assert!(body.contains("if (slot >= LOCAL_SUB_TABLE_CAPACITY) return NULL"));
    assert!(body.contains("if (!local_sub_table_bitmap_get_(self, slot)) return NULL"));
    assert!(
        !body.contains("self->generation[slot]"),
        "C11 get_by_slot must NOT do generation check; body:\n{body}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Cross-backend invariants ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn get_by_slot_emits_on_all_six_backends() {
    // Drift guard: every backend's BC emit must surface a
    // `get_by_slot` (or `GetBySlot` / `getBySlot`) method per the
    // C7-foundation contract. A regression that drops the method
    // on one backend would fail this guard before the C7-lowering
    // atomic catches the missing dispatch target.
    for lang in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let code = compile_bc_for(lang);
        let needle = match lang {
            Language::Go => "GetBySlot",
            Language::Kotlin => "getBySlot",
            // Rust / Cpp / Python / C11 share the snake_case form.
            _ => "get_by_slot",
        };
        assert!(
            code.contains(needle),
            "{:?}: missing `{}` method emit; got:\n{}",
            lang,
            needle,
            code
        );
    }
}

#[test]
fn get_by_slot_does_not_disturb_existing_get_handle_path() {
    // Drift guard: introducing `get_by_slot` must NOT alter the
    // existing Handle-based `get(handle)` emit. Catches a refactor
    // regression that accidentally folded the two methods.
    for lang in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let code = compile_bc_for(lang);
        // Each backend's Handle-based get still emits with its
        // generation check inline (the use-after-remove guard).
        let gen_needle = match lang {
            Language::Rust => "self.generation[slot] != handle.generation()",
            Language::Cpp => "generation_[slot] != handle.generation()",
            Language::Kotlin => "(generation[slotIdx].toUInt() and GEN_MASK) != handle.generation",
            Language::Go => "t.generation[slot] != gen",
            Language::Python => "self._gen_get(slot) != gen",
            Language::C11 => "self->generation[slot] != gen",
        };
        assert!(
            code.contains(gen_needle),
            "{:?}: Handle-based get's generation check disturbed; expected `{}` in emit",
            lang,
            gen_needle,
        );
    }
}
