//! C6-γ3 — Bounded-collection Cpp + Kotlin template emit
//! integration tests.
//!
//! Per watching-zenoh RFC §5.L lines 2540-2655, the γ3 atomic ships
//! the 2nd + 3rd backends (Cpp + Kotlin) for `<scxml sce:kind=
//! "bounded-collection">`. Both backends reuse the
//! [`BoundedCollectionResolution`] resolution bundle threaded by
//! the orchestrator that γ2 introduced, swapping the abstract
//! `index_by_field_sce_type` for their per-language type string at
//! render time via the existing `cpp_type` / `kotlin_type` helpers.
//!
//! Test strategy follows γ2's `c6_bounded_collection_gamma2.rs`:
//! emit-shape grep against spec-locked emit invariants is cheaper
//! than standing up `g++` / `kotlinc` and catches the template-
//! layer regressions that matter for codegen correctness.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

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

fn bc_doc(name: &str, element_type: &str, capacity: u32, extras: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="{name}" version="1.0">
  <sce:element-type>{element_type}</sce:element-type>
  <sce:capacity const="{capacity}"/>{extras}
</scxml>"##
    )
}

fn compile_pair_for(
    lang: Language,
    bc_xml: &str,
    codec_xml: &str,
    codec_basename: &str,
    bc_basename: &str,
) -> String {
    let dir = tempdir().expect("tempdir");
    let codec = write_doc(dir.path(), codec_basename, codec_xml);
    let bc = write_doc(dir.path(), bc_basename, bc_xml);
    let outputs = compile_scxml_with_imports(
        &[],
        &[codec.as_path(), bc.as_path()],
        &template_dir(lang),
        lang,
        &ForgeCompileOptions::default(),
    )
    .expect("orchestrator codegen succeeds");
    let bc_output = outputs
        .iter()
        .find(|(name, _)| name == bc_basename)
        .unwrap_or_else(|| {
            panic!(
                "BC output not present; got entries: {:?}",
                outputs.iter().map(|(n, _)| n).collect::<Vec<_>>()
            )
        });
    bc_output
        .1
        .files
        .iter()
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ═══════════════════════════════════════════════════════════════
// ── Cpp backend ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn cpp_happy_compile_const_no_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Cpp,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Header guard + namespace.
    assert!(code.contains("#ifndef SCE_FORGE_LOCAL_SUB_TABLE_H"));
    assert!(code.contains("namespace SCE::Generated::LocalSubTable {"));

    // Element-type cross-doc include (spec line 2566-2567).
    assert!(code.contains("#include \"subscription_entry.h\""));
    assert!(code.contains(
        "using SubscriptionEntryType = ::SCE::Generated::SubscriptionEntry::SubscriptionEntry;"
    ));

    // Capacity literal (spec line 2583-2585).
    assert!(code.contains("inline constexpr std::size_t CAPACITY = 8;"));

    // POD Handle struct + 16/16 split (Q-γ3-Handle-cpp-shape (a)).
    assert!(code.contains("struct LocalSubTableHandle {"));
    assert!(code.contains("std::uint32_t raw;"));
    assert!(code.contains("inline constexpr std::uint32_t SLOT_BITS = 16;"));
    assert!(code.contains("inline constexpr std::uint32_t GEN_BITS = 16;"));

    // std::array + std::bitset (spec line 2576-2577).
    assert!(code.contains("std::array<SubscriptionEntryType, CAPACITY> slots_{};"));
    assert!(code.contains("std::bitset<CAPACITY> bitmap_;"));
    assert!(code.contains("std::array<std::uint32_t, CAPACITY> generation_{};"));

    // Operations contract (spec lines 2609-2619).
    assert!(code.contains(
        "LocalSubTableInsertResult insert(const SubscriptionEntryType& elem)"
    ));
    assert!(code.contains("bool remove(LocalSubTableHandle handle)"));
    assert!(code.contains(
        "std::optional<SubscriptionEntryType> get(LocalSubTableHandle handle) const"
    ));
    assert!(code.contains("static constexpr std::size_t capacity() noexcept"));
    assert!(code.contains("std::size_t len() const noexcept"));

    // No find_by_index method (no <sce:index-by>).
    assert!(
        !code.contains("find_by_index("),
        "no <sce:index-by> means no find_by_index method emit"
    );

    // Iterator emit per spec line 2616 — begin()/end() pair.
    assert!(code.contains("class const_iterator {"));
    assert!(code.contains("const_iterator begin() const"));
    assert!(code.contains("const_iterator end() const"));
}

#[test]
fn cpp_happy_compile_const_with_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        16,
        "\n  <sce:index-by field=\"key_expr_id\"/>",
    );
    let code = compile_pair_for(
        Language::Cpp,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // find_by_index emits with const-ref uint32_t key (spec line
    // 2615 + cpp_type(SceType::Uint32) = "uint32_t").
    assert!(
        code.contains(
            "std::optional<LocalSubTableHandle> find_by_index(const uint32_t& key) const"
        ),
        "expected typed find_by_index over const uint32_t&; got:\n{code}"
    );
    // The field name appears in the comparison body.
    assert!(
        code.contains("slots_[slot].key_expr_id == key"),
        "expected `slots_[slot].key_expr_id == key` comparison body"
    );
}

#[test]
fn cpp_overflow_reject_emits_err_return() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>reject</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Cpp,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Err path: InsertResult { false, ... , OverflowError{} }
    assert!(
        code.contains("LocalSubTableInsertResult{") && code.contains("false,"),
        "reject policy must emit InsertResult{{ false, ..., OverflowError{{}} }}; got:\n{code}"
    );
    // No eviction helper for reject.
    assert!(
        !code.contains("oldest_occupied_slot"),
        "reject policy must NOT emit `oldest_occupied_slot`"
    );
}

#[test]
fn cpp_overflow_oldest_wins_evicts() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>oldest-wins</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Cpp,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert!(
        code.contains("std::size_t oldest_occupied_slot() const"),
        "oldest-wins must emit `oldest_occupied_slot` helper"
    );
    assert!(
        code.contains("generation_[slot] = (generation_[slot] + 1) & GEN_MASK;"),
        "oldest-wins must increment the slot's generation counter"
    );
}

#[test]
fn cpp_use_after_remove_branches_present() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Cpp,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    let needle = "if (generation_[slot] != handle.generation()) ";
    let occurrences = code.matches(needle).count();
    assert!(
        occurrences >= 2,
        "expected generation check in both remove() and get() (>= 2); found {occurrences}"
    );
    // Increment-on-free in remove().
    assert!(code.contains("generation_[slot] = (generation_[slot] + 1) & GEN_MASK;"));
}

// ═══════════════════════════════════════════════════════════════
// ── Kotlin backend ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn kotlin_happy_compile_const_no_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Kotlin,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Package + cross-doc import.
    assert!(code.contains("package com.sce.generated.local_sub_table"));
    assert!(code.contains("import com.sce.generated.subscription_entry.SubscriptionEntry"));

    // Capacity literal.
    assert!(code.contains("const val CAPACITY: Int = 8"));

    // @JvmInline value class Handle (Q-γ3-Handle-kotlin-shape (a)).
    assert!(code.contains("@JvmInline"));
    assert!(code.contains("value class LocalSubTableHandle(val raw: UInt)"));
    assert!(code.contains("const val SLOT_BITS: Int = 16"));
    assert!(code.contains("const val GEN_BITS: Int = 16"));

    // Spec line 2578: Array<T?>(N) + BooleanArray(N) + IntArray(N).
    assert!(code.contains("private val slots: Array<SubscriptionEntry?> = arrayOfNulls(CAPACITY)"));
    assert!(code.contains("private val occupied: BooleanArray = BooleanArray(CAPACITY)"));
    assert!(code.contains("private val generation: IntArray = IntArray(CAPACITY)"));

    // Iterable<T> conformance (spec line 2616).
    assert!(code.contains("class LocalSubTable : Iterable<SubscriptionEntry>"));
    assert!(code.contains("override fun iterator(): Iterator<SubscriptionEntry>"));

    // Operations contract.
    assert!(code.contains("fun insert(elem: SubscriptionEntry): LocalSubTableInsertResult"));
    assert!(code.contains("fun remove(handle: LocalSubTableHandle): Boolean"));
    assert!(code.contains("fun get(handle: LocalSubTableHandle): SubscriptionEntry?"));
    assert!(code.contains("fun capacity(): Int = CAPACITY"));
    assert!(code.contains("fun len(): Int = count"));

    // No findByIndex method (no <sce:index-by>).
    assert!(
        !code.contains("fun findByIndex("),
        "no <sce:index-by> means no findByIndex method emit"
    );
}

#[test]
fn kotlin_happy_compile_const_with_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        16,
        "\n  <sce:index-by field=\"key_expr_id\"/>",
    );
    let code = compile_pair_for(
        Language::Kotlin,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // findByIndex with `UInt` key (kotlin_type(SceType::Uint32) = "UInt").
    assert!(
        code.contains("fun findByIndex(key: UInt): LocalSubTableHandle?"),
        "expected findByIndex(key: UInt); got:\n{code}"
    );
    assert!(
        code.contains("elem.key_expr_id == key"),
        "expected `elem.key_expr_id == key` comparison body"
    );
}

#[test]
fn kotlin_overflow_reject_emits_err_return() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>reject</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Kotlin,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert!(
        code.contains("return LocalSubTableInsertResult.Err"),
        "reject policy must return InsertResult.Err; got:\n{code}"
    );
    assert!(
        !code.contains("oldestOccupiedSlot"),
        "reject policy must NOT emit `oldestOccupiedSlot`"
    );
}

#[test]
fn kotlin_overflow_oldest_wins_evicts() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>oldest-wins</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Kotlin,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert!(
        code.contains("private fun oldestOccupiedSlot(): Int"),
        "oldest-wins must emit `oldestOccupiedSlot` helper"
    );
    assert!(
        code.contains("((generation[evictSlot].toUInt() + 1u) and GEN_MASK).toInt()"),
        "oldest-wins must increment the slot's generation counter"
    );
}

#[test]
fn kotlin_use_after_remove_branches_present() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Kotlin,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    let needle = "if ((generation[slotIdx].toUInt() and GEN_MASK) != handle.generation)";
    let occurrences = code.matches(needle).count();
    assert!(
        occurrences >= 2,
        "expected generation check in both remove() and get() (>= 2); found {occurrences}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Cross-backend invariants ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn capacity_literal_sweep_all_backends() {
    // CompileConst N → CAPACITY emit literal for every backend that
    // ships a γ-stage template. Sweep includes edge values (1) and
    // common bounded-collection sizes (8, 64, 1024).
    for n in [1u32, 8, 64, 1024] {
        let codec = codec_doc("subscription_entry");
        let bc = bc_doc("local_sub_table", "subscription_entry", n, "");

        let cpp = compile_pair_for(
            Language::Cpp,
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert!(
            cpp.contains(&format!("inline constexpr std::size_t CAPACITY = {n};")),
            "cpp CAPACITY literal {n}: not in emit\n{cpp}"
        );

        let kt = compile_pair_for(
            Language::Kotlin,
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert!(
            kt.contains(&format!("const val CAPACITY: Int = {n}")),
            "kotlin CAPACITY literal {n}: not in emit\n{kt}"
        );
    }
}

#[test]
fn cpp_kotlin_element_type_procedure_emits() {
    // Procedure element-type with a `ulong` internal field exercises
    // SceType::Uint64 → cpp_type "uint64_t" / kotlin_type "ULong"
    // dispatch via the shared resolver's abstract SceType plumbing.
    let procedure = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="frame_record" version="1.0" initial="run">
  <datamodel>
    <data id="serial" sce:type="uint32" sce:direction="in"/>
    <data id="checksum" sce:type="uint64" sce:direction="internal" expr="0"/>
  </datamodel>
  <state id="run">
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>"##;
    let bc = bc_doc(
        "reassembly_table",
        "frame_record",
        4,
        "\n  <sce:index-by field=\"checksum\"/>",
    );

    let dir = tempdir().expect("tempdir");
    let proc_path = write_doc(dir.path(), "frame_record.scxml", procedure);
    let bc_path = write_doc(dir.path(), "reassembly_table.scxml", &bc);

    // Cpp emit: `find_by_index(const uint64_t& key)`.
    let cpp_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Cpp),
        Language::Cpp,
        &ForgeCompileOptions::default(),
    )
    .expect("cpp orchestrator codegen succeeds");
    let cpp_body = cpp_outputs
        .iter()
        .find(|(name, _)| name == "reassembly_table.scxml")
        .expect("cpp BC output present")
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n");
    assert!(cpp_body.contains("#include \"frame_record.h\""));
    assert!(cpp_body.contains(
        "std::optional<ReassemblyTableHandle> find_by_index(const uint64_t& key) const"
    ));

    // Kotlin emit: `findByIndex(key: ULong)`.
    let kt_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Kotlin),
        Language::Kotlin,
        &ForgeCompileOptions::default(),
    )
    .expect("kotlin orchestrator codegen succeeds");
    let kt_body = kt_outputs
        .iter()
        .find(|(name, _)| name == "reassembly_table.scxml")
        .expect("kotlin BC output present")
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n");
    assert!(kt_body.contains("import com.sce.generated.frame_record.FrameRecord"));
    assert!(kt_body.contains("fun findByIndex(key: ULong): ReassemblyTableHandle?"));
}

#[test]
fn template_ships_matrix_flips() {
    // Drift guard: γ3 flips `(Cpp, *)` + `(Kotlin, *)` to `true`
    // for `ForgeKind::BoundedCollection` while leaving `(Go, *)` /
    // `(Python, *)` / `(C11, *)` as `false` (γ4 scope). The matrix
    // check is via the public lookup function — a regression that
    // flips Go/Python/C11 early would fail this guard before any
    // emit fixture catches the broken-template fallout.
    use sce_build::forge::codegen_matrix::{lookup, EmitOutcome};
    use sce_build::forge::model::ForgeKind;
    assert_eq!(
        lookup(ForgeKind::BoundedCollection, Language::Rust),
        EmitOutcome::Emit
    );
    assert_eq!(
        lookup(ForgeKind::BoundedCollection, Language::Cpp),
        EmitOutcome::Emit
    );
    assert_eq!(
        lookup(ForgeKind::BoundedCollection, Language::Kotlin),
        EmitOutcome::Emit
    );
    assert_eq!(
        lookup(ForgeKind::BoundedCollection, Language::Go),
        EmitOutcome::TemplateMissing
    );
    assert_eq!(
        lookup(ForgeKind::BoundedCollection, Language::Python),
        EmitOutcome::TemplateMissing
    );
    assert_eq!(
        lookup(ForgeKind::BoundedCollection, Language::C11),
        EmitOutcome::TemplateMissing
    );
}
