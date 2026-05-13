//! C6-γ4 — Bounded-collection Go + Python + C11 template emit
//! integration tests.
//!
//! Per watching-zenoh RFC §5.L lines 2540-2655, the γ4 atomic ships
//! the 4th, 5th, and 6th backends (Go + Python + C11) for `<scxml
//! sce:kind="bounded-collection">`, entirely closing the §5.L
//! 6-backend codegen matrix. All three backends reuse the
//! [`BoundedCollectionResolution`] resolution bundle threaded by
//! the orchestrator that γ2 introduced, swapping the abstract
//! `index_by_field_sce_type` for their per-language type string at
//! render time via the lifted `go_type` / `python_type` / `c_type`
//! helpers.
//!
//! Test strategy mirrors γ2/γ3: emit-shape grep against spec-locked
//! invariants is cheaper than standing up `go vet` / `python -c` /
//! `gcc` and catches the template-layer regressions that matter for
//! codegen correctness.
//!
//! Design Q's locked 2026-05-13 (Q-γ4-{Go-iter, Go-Handle-method-set,
//! Python-Handle-shape, Python-overflow-emit, C11-iter-shape}) =
//! (a) textbook recommendations:
//!   - Go: `ForEach(fn func(T))` callback + receiver methods on
//!     `uint32` newtype Handle.
//!   - Python: frozen `@dataclass(slots=True)` Handle + `Optional
//!     [Handle]` overflow return.
//!   - C11: `<snake>_foreach(self, fn, user)` callback iteration.
//! All five lock-ins preserve ABI parity of the 16/16 packed
//! `uint32_t` Handle across the full 6-backend matrix.

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

fn options_for(lang: Language) -> ForgeCompileOptions {
    // Go BC's element-type cross-package import requires
    // `go_module_prefix`; the C11 + Python paths are self-contained
    // (relative-package import for Python, sibling `#include` for
    // C11). Set unconditionally so the same `options_for` helper
    // works across all three backends.
    let mut opts = ForgeCompileOptions::default();
    if matches!(lang, Language::Go) {
        opts.go_module_prefix = Some(GO_PREFIX.to_string());
    }
    opts
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
        &options_for(lang),
        None,
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
// ── C11 backend ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn c11_happy_compile_const_no_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::C11,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Header guard + sibling include (spec line 2566-2567).
    assert!(code.contains("#ifndef SCE_FORGE_LOCAL_SUB_TABLE_H"));
    assert!(code.contains("#include \"subscription_entry.h\""));

    // Capacity literal + drift guard (spec line 2575 + 2583-2585).
    assert!(code.contains("#define LOCAL_SUB_TABLE_CAPACITY ((uint32_t)8)"));
    assert!(code.contains("_Static_assert(8 <= UINT16_MAX,"));

    // POD Handle struct + 16/16 split (Q-γ4-C11 ABI parity).
    assert!(code.contains("typedef struct {\n    uint32_t raw;\n} local_sub_table_handle_t;"));
    assert!(code.contains("#define LOCAL_SUB_TABLE_SLOT_BITS 16u"));
    assert!(code.contains("#define LOCAL_SUB_TABLE_GEN_BITS 16u"));

    // Storage shape per spec lines 2573-2575 verbatim. The element-
    // type reference uses the codec's emitted typedef
    // `<element_snake>_t` (C7-lowering 2026-05-13 foundation fix —
    // pre-C7-lowering the BC C11 template referenced `<element_pascal>`
    // which has no codec-side typedef, making gcc compilation fail
    // silently under the text-only foundation tests).
    assert!(code.contains("typedef struct {\n    subscription_entry_t slots[LOCAL_SUB_TABLE_CAPACITY];"));
    assert!(code.contains("uint32_t generation[LOCAL_SUB_TABLE_CAPACITY];"));
    assert!(code.contains("uint32_t bitmap[LOCAL_SUB_TABLE_BITMAP_WORDS];"));
    assert!(code.contains("uint32_t count;\n} local_sub_table_t;"));

    // Operations contract (spec line 2575 snake_case API).
    assert!(code.contains("static inline local_sub_table_insert_result_t local_sub_table_insert("));
    assert!(code.contains("static inline bool local_sub_table_remove("));
    assert!(code.contains("static inline const subscription_entry_t *local_sub_table_get("));
    assert!(code.contains("static inline uint32_t local_sub_table_len("));
    assert!(code.contains("static inline uint32_t local_sub_table_capacity("));

    // Q-γ4-C11-iter-shape (a): callback iteration.
    assert!(code.contains("static inline void local_sub_table_foreach("));
    assert!(code.contains("void (*fn)(const subscription_entry_t *elem, void *user),"));

    // No find_by_index without <sce:index-by>.
    assert!(
        !code.contains("local_sub_table_find_by_index("),
        "no <sce:index-by> means no _find_by_index emit"
    );
}

#[test]
fn c11_happy_compile_const_with_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        16,
        "\n  <sce:index-by field=\"key_expr_id\"/>",
    );
    let code = compile_pair_for(
        Language::C11,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // find_by_index emits with uint32_t key (c_type(Uint32) = "uint32_t").
    assert!(
        code.contains("static inline local_sub_table_insert_result_t local_sub_table_find_by_index(\n    const local_sub_table_t *self,\n    uint32_t key)"),
        "expected typed _find_by_index over uint32_t key; got:\n{code}"
    );
    // Snake-case field reference matches C11 codec emit.
    assert!(
        code.contains("self->slots[slot].key_expr_id == key"),
        "expected `slots[slot].key_expr_id == key` comparison body"
    );
}

#[test]
fn c11_overflow_reject_emits_err_return() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>reject</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::C11,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Reject path: r.ok = false; r.handle.raw = 0u; return r;
    assert!(
        code.contains("r.ok = false;\n    r.handle.raw = 0u;"),
        "reject policy must zero-out the handle and set ok=false; got:\n{code}"
    );
    // No eviction code path.
    assert!(
        !code.contains("self->generation[0] = (self->generation[0] + 1u)"),
        "reject policy must NOT emit slot-0 generation bump (oldest-wins only)"
    );
}

#[test]
fn c11_overflow_oldest_wins_evicts() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>oldest-wins</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::C11,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Eviction body lives in insert().
    assert!(
        code.contains("self->slots[0] = *elem;"),
        "oldest-wins must overwrite slot 0"
    );
    assert!(
        code.contains("self->generation[0] = (self->generation[0] + 1u) & LOCAL_SUB_TABLE_GEN_MASK;"),
        "oldest-wins must increment slot 0's generation counter"
    );
}

#[test]
fn c11_use_after_remove_branches_present() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::C11,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    let needle = "if (self->generation[slot] != gen) return";
    let occurrences = code.matches(needle).count();
    assert!(
        occurrences >= 2,
        "expected generation check in both _remove() and _get() (>= 2); found {occurrences}"
    );
    // Increment-on-free in remove().
    assert!(code.contains(
        "self->generation[slot] = (self->generation[slot] + 1u) & LOCAL_SUB_TABLE_GEN_MASK;"
    ));
}

// ═══════════════════════════════════════════════════════════════
// ── Go backend ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn go_happy_compile_const_no_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Go,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Package + cross-package element-type import (qualified path).
    assert!(code.contains("package local_sub_table"));
    assert!(code.contains(&format!(
        "\"{GO_PREFIX}/subscription_entry\""
    )));

    // Capacity literal.
    assert!(code.contains("const LocalSubTableCapacity = 8"));
    assert!(code.contains("const _ = uint16(8)"));

    // Q-γ4-Go-Handle-method-set (a): receiver methods on uint32 newtype.
    assert!(code.contains("type LocalSubTableHandle uint32"));
    assert!(code.contains("func (h LocalSubTableHandle) Slot() uint32 {"));
    assert!(code.contains("func (h LocalSubTableHandle) Generation() uint32 {"));
    assert!(code.contains("func NewLocalSubTableHandle(slot, generation uint32) LocalSubTableHandle {"));
    assert!(code.contains("const LocalSubTableSlotBits uint32 = 16"));
    assert!(code.contains("const LocalSubTableGenBits uint32 = 16"));

    // Storage shape per spec line 2579.
    assert!(code.contains("slots      [LocalSubTableCapacity]subscription_entry.SubscriptionEntry"));
    assert!(code.contains("generation [LocalSubTableCapacity]uint32"));
    assert!(code.contains("occupied   [LocalSubTableCapacity]bool"));
    assert!(code.contains("count      int"));

    // Operations contract.
    assert!(code.contains(
        "func (t *LocalSubTable) Insert(elem subscription_entry.SubscriptionEntry) (LocalSubTableHandle, error) {"
    ));
    assert!(code.contains("func (t *LocalSubTable) Remove(handle LocalSubTableHandle) bool {"));
    assert!(code.contains(
        "func (t *LocalSubTable) Get(handle LocalSubTableHandle) (*subscription_entry.SubscriptionEntry, bool) {"
    ));
    assert!(code.contains("func (t *LocalSubTable) Len() int {"));
    assert!(code.contains("func (t *LocalSubTable) Capacity() int {"));

    // Q-γ4-Go-iter-shape (a): ForEach callback.
    assert!(code.contains(
        "func (t *LocalSubTable) ForEach(fn func(subscription_entry.SubscriptionEntry)) {"
    ));

    // No FindByIndex without <sce:index-by>.
    assert!(
        !code.contains("func (t *LocalSubTable) FindByIndex("),
        "no <sce:index-by> means no FindByIndex emit"
    );
}

#[test]
fn go_happy_compile_const_with_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        16,
        "\n  <sce:index-by field=\"key_expr_id\"/>",
    );
    let code = compile_pair_for(
        Language::Go,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // FindByIndex with uint32 key (go_type(Uint32) = "uint32").
    assert!(
        code.contains(
            "func (t *LocalSubTable) FindByIndex(key uint32) (LocalSubTableHandle, bool) {"
        ),
        "expected FindByIndex(key: uint32); got:\n{code}"
    );
    // PascalCase field reference (matches Go codec's `codec_field_id`).
    assert!(
        code.contains("t.slots[slot].KeyExprId == key"),
        "expected `t.slots[slot].KeyExprId == key` comparison body"
    );
}

#[test]
fn go_overflow_reject_emits_err_return() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>reject</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Go,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Sentinel error declared + reject path returns it.
    assert!(code.contains(
        "var ErrLocalSubTableOverflow = errors.New(\n\t\"local_sub_table: capacity 4 exhausted\")"
    ));
    assert!(
        code.contains("return LocalSubTableHandle(0), ErrLocalSubTableOverflow"),
        "reject policy must return (Handle(0), Err...Overflow); got:\n{code}"
    );
}

#[test]
fn go_overflow_oldest_wins_evicts() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>oldest-wins</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Go,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert!(
        code.contains("t.slots[0] = elem"),
        "oldest-wins must overwrite slot 0"
    );
    assert!(
        code.contains("t.generation[0] = (t.generation[0] + 1) & LocalSubTableGenMask"),
        "oldest-wins must increment slot 0's generation counter"
    );
    // Returns nil error on eviction (always-succeeds branch).
    assert!(
        code.contains("return NewLocalSubTableHandle(0, t.generation[0]), nil"),
        "oldest-wins must return a fresh handle with nil error"
    );
}

#[test]
fn go_use_after_remove_branches_present() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Go,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    let needle = "if t.generation[slot] != gen {";
    let occurrences = code.matches(needle).count();
    assert!(
        occurrences >= 2,
        "expected generation check in both Remove() and Get() (>= 2); found {occurrences}"
    );
    assert!(code.contains(
        "t.generation[slot] = (t.generation[slot] + 1) & LocalSubTableGenMask"
    ));
}

#[test]
fn go_missing_module_prefix_errors() {
    // BC's Go render requires `go_module_prefix` for the element-type
    // cross-package import. Missing prefix must surface InvalidConfig
    // (parity with `validate_options` for the `<sce:import>` path).
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");

    let dir = tempdir().expect("tempdir");
    let codec_path = write_doc(dir.path(), "subscription_entry.scxml", &codec);
    let bc_path = write_doc(dir.path(), "local_sub_table.scxml", &bc);

    let result = compile_scxml_with_imports(
        &[],
        &[codec_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Go),
        Language::Go,
        &ForgeCompileOptions::default(), // no go_module_prefix
        None,
    );
    let err = match result {
        Ok(_) => panic!("Go BC without go_module_prefix must error"),
        Err(e) => e,
    };

    let msg = format!("{err}");
    assert!(
        msg.contains("go_module_prefix"),
        "error must name the missing field; got: {msg}"
    );
    assert!(
        msg.contains("bounded-collection 'local_sub_table'"),
        "error must name the offending BC; got: {msg}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Python backend ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[test]
fn python_happy_compile_const_no_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Python,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Relative-package element-type import.
    assert!(code.contains("from .subscription_entry import SubscriptionEntry"));

    // Capacity literal + drift guard.
    assert!(code.contains("CAPACITY: int = 8"));
    assert!(code.contains("assert CAPACITY <= 0xFFFF,"));

    // Q-γ4-Python-Handle-shape (a): frozen @dataclass(slots=True).
    assert!(code.contains("@dataclasses.dataclass(frozen=True, slots=True)"));
    assert!(code.contains("class LocalSubTableHandle:"));
    assert!(code.contains("raw: int"));
    assert!(code.contains("def pack(cls, slot: int, generation: int) -> \"LocalSubTableHandle\":"));

    // Class scaffolding.
    assert!(code.contains("class LocalSubTable:"));
    assert!(code.contains("__slots__ = (\"_slots\", \"_generation\", \"_occupied\", \"_count\")"));

    // Q-γ4-Python-overflow-emit (a): Optional[Handle] (None on reject).
    assert!(code.contains(
        "def insert(self, elem: SubscriptionEntry) -> Optional[LocalSubTableHandle]:"
    ));
    assert!(code.contains("def remove(self, handle: LocalSubTableHandle) -> bool:"));
    assert!(code.contains(
        "def get(self, handle: LocalSubTableHandle) -> Optional[SubscriptionEntry]:"
    ));
    assert!(code.contains("def __iter__(self) -> Iterator[SubscriptionEntry]:"));
    assert!(code.contains("def for_each(self, fn: Callable[[SubscriptionEntry], None]) -> None:"));

    // No find_by_index without <sce:index-by>.
    assert!(
        !code.contains("def find_by_index("),
        "no <sce:index-by> means no find_by_index emit"
    );
}

#[test]
fn python_happy_compile_const_with_index_by() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        16,
        "\n  <sce:index-by field=\"key_expr_id\"/>",
    );
    let code = compile_pair_for(
        Language::Python,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // find_by_index with int key (python_type(Uint32) = "int").
    assert!(
        code.contains(
            "def find_by_index(self, key: int) -> Optional[LocalSubTableHandle]:"
        ),
        "expected find_by_index(key: int); got:\n{code}"
    );
    // Snake_case field reference (matches Python codec's snake_case emit).
    assert!(
        code.contains("elem.key_expr_id == key"),
        "expected `elem.key_expr_id == key` comparison body"
    );
}

#[test]
fn python_overflow_reject_returns_none() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>reject</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Python,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    // Reject path: `return None`.
    assert!(
        code.contains("return None"),
        "reject policy must `return None`; got:\n{code}"
    );
    // No eviction (no slot-0 overwrite under reject).
    assert!(
        !code.contains("self._slots[0] = elem"),
        "reject policy must NOT emit slot-0 overwrite (oldest-wins only)"
    );
}

#[test]
fn python_overflow_oldest_wins_evicts() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>oldest-wins</sce:on-overflow>",
    );
    let code = compile_pair_for(
        Language::Python,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert!(
        code.contains("self._slots[0] = elem"),
        "oldest-wins must overwrite slot 0"
    );
    assert!(
        code.contains("self._gen_set(0, (self._gen_get(0) + 1) & _GEN_MASK)"),
        "oldest-wins must increment slot 0's generation counter"
    );
}

#[test]
fn python_use_after_remove_branches_present() {
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair_for(
        Language::Python,
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    let needle = "if self._gen_get(slot) != gen:";
    let occurrences = code.matches(needle).count();
    assert!(
        occurrences >= 2,
        "expected generation check in both remove() and get() (>= 2); found {occurrences}"
    );
    assert!(code.contains(
        "self._gen_set(slot, (self._gen_get(slot) + 1) & _GEN_MASK)"
    ));
}

// ═══════════════════════════════════════════════════════════════
// ── Cross-backend invariants (matrix-closure drift guards) ─────
// ═══════════════════════════════════════════════════════════════

#[test]
fn capacity_literal_sweep_all_six_backends() {
    // CompileConst N → CAPACITY emit literal for every backend.
    // The γ chain closure means all 6 backends emit; this is the
    // canonical drift guard that catches per-backend template
    // regressions in the capacity-substitution path.
    for n in [1u32, 8, 64, 1024] {
        let codec = codec_doc("subscription_entry");
        let bc = bc_doc("local_sub_table", "subscription_entry", n, "");

        let rust = compile_pair_for(
            Language::Rust,
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert!(
            rust.contains(&format!("pub const CAPACITY: usize = {n};")),
            "rust CAPACITY literal {n}: not in emit\n{rust}"
        );

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

        let go = compile_pair_for(
            Language::Go,
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert!(
            go.contains(&format!("const LocalSubTableCapacity = {n}")),
            "go CAPACITY literal {n}: not in emit\n{go}"
        );

        let py = compile_pair_for(
            Language::Python,
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert!(
            py.contains(&format!("CAPACITY: int = {n}")),
            "python CAPACITY literal {n}: not in emit\n{py}"
        );

        let c11 = compile_pair_for(
            Language::C11,
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert!(
            c11.contains(&format!("#define LOCAL_SUB_TABLE_CAPACITY ((uint32_t){n})")),
            "c11 CAPACITY literal {n}: not in emit\n{c11}"
        );
    }
}

#[test]
fn procedure_element_type_all_six_backends() {
    // Procedure element-type with a `uint64` internal field exercises
    // SceType::Uint64 → per-language type dispatch via the shared
    // resolver's abstract SceType plumbing. All 6 backends must
    // surface a `find_by_index`-style API typed against the resolved
    // u64 column.
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

    let extract_bc = |outputs: &[(String, GeneratedOutput)]| -> String {
        outputs
            .iter()
            .find(|(name, _)| name == "reassembly_table.scxml")
            .expect("BC output present")
            .1
            .files
            .iter()
            .map(|(_, c)| c.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    // C11: const local_sub_table_t *self, uint64_t key.
    let c11_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::C11),
        Language::C11,
        &options_for(Language::C11),
        None,
    )
    .expect("c11 codegen");
    let c11_body = extract_bc(&c11_outputs);
    assert!(c11_body.contains("#include \"frame_record.h\""));
    assert!(c11_body.contains(
        "static inline reassembly_table_insert_result_t reassembly_table_find_by_index(\n    const reassembly_table_t *self,\n    uint64_t key)"
    ));

    // Go: FindByIndex(key uint64).
    let go_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Go),
        Language::Go,
        &options_for(Language::Go),
        None,
    )
    .expect("go codegen");
    let go_body = extract_bc(&go_outputs);
    assert!(go_body.contains(&format!("\"{GO_PREFIX}/frame_record\"")));
    assert!(go_body.contains(
        "func (t *ReassemblyTable) FindByIndex(key uint64) (ReassemblyTableHandle, bool) {"
    ));

    // Python: find_by_index(self, key: int).
    let py_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Python),
        Language::Python,
        &options_for(Language::Python),
        None,
    )
    .expect("python codegen");
    let py_body = extract_bc(&py_outputs);
    assert!(py_body.contains("from .frame_record import FrameRecord"));
    assert!(
        py_body.contains(
            "def find_by_index(self, key: int) -> Optional[ReassemblyTableHandle]:"
        )
    );

    // Cpp (γ3 sanity): const uint64_t& key.
    let cpp_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Cpp),
        Language::Cpp,
        &options_for(Language::Cpp),
        None,
    )
    .expect("cpp codegen");
    let cpp_body = extract_bc(&cpp_outputs);
    assert!(cpp_body.contains(
        "std::optional<ReassemblyTableHandle> find_by_index(const uint64_t& key) const"
    ));

    // Kotlin (γ3 sanity): findByIndex(key: ULong).
    let kt_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Kotlin),
        Language::Kotlin,
        &options_for(Language::Kotlin),
        None,
    )
    .expect("kotlin codegen");
    let kt_body = extract_bc(&kt_outputs);
    assert!(kt_body.contains("fun findByIndex(key: ULong): ReassemblyTableHandle?"));

    // Rust (γ2 sanity): fn find_by_index(&self, key: u64).
    let rust_outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(Language::Rust),
        Language::Rust,
        &options_for(Language::Rust),
        None,
    )
    .expect("rust codegen");
    let rust_body = extract_bc(&rust_outputs);
    assert!(rust_body.contains("pub fn find_by_index(&self, key: &u64)"));
}

#[test]
fn template_ships_matrix_fully_closed() {
    // γ chain closure drift guard. After γ4, all 6 backends emit
    // for `ForgeKind::BoundedCollection` — the `template_ships`
    // matrix is fully closed. A regression that flips any backend
    // back to `false` would fail this guard before any emit fixture
    // catches the broken-template fallout.
    use sce_build::forge::codegen_matrix::{lookup, EmitOutcome};
    use sce_build::forge::model::ForgeKind;
    for lang in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        assert_eq!(
            lookup(ForgeKind::BoundedCollection, lang),
            EmitOutcome::Emit,
            "BoundedCollection must emit on {:?} after γ chain closure",
            lang
        );
    }
}
