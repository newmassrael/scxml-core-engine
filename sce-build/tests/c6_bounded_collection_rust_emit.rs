//! Bounded-collection Rust template emit integration tests.
//!
//! Per SCE Protocol-Synthesis RFC §synth-5-L lines 2540-2655, this suite covers
//! the Rust backend emit for `<scxml sce:kind="bounded-
//! collection">`. The template emits a slot table over
//! `Vec<Option<T>>` (std) / `heapless::Vec<Option<T>, N>` (no_std),
//! a `[u32; CAPACITY]` generation array, a `[u32; (CAPACITY+31)/32]`
//! occupancy bitmap, and the operations contract from spec lines
//! 2609-2619 (`insert` / `remove` / `get` / `find_by_index` /
//! `iter` / `len` / `capacity`). Handle is a tuple newtype over
//! `u32` carrying slot index (low 16 bits) + generation counter
//! (high 16 bits) per spec lines 2621-2622.
//!
//! Test strategy follows the existing `rust_golden_syn_gate`
//! precedent in `forge_conformance.rs`: parse the emitted Rust
//! through `syn::parse_file` to confirm syntactic validity, then
//! grep for emit-shape invariants. This is cheaper than standing
//! up rustc and catches the template-layer regressions that matter
//! for codegen correctness.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_forge_with_deploy;
use sce_build::compile_scxml_with_imports;
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::DocumentLabel;
use sce_build::ForgeCompileOptions;

fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

fn write_doc(dir: &Path, basename: &str, content: &str) -> PathBuf {
    let path = dir.join(basename);
    fs::write(&path, content).expect("write doc");
    path
}

/// Two-field codec used as element-type. The `key_expr_id: uint32`
/// field is the index-by target in the `with_index_by` scenarios.
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

/// Bounded-collection doc builder. The `extras` slot lets each test
/// inject `<sce:index-by>` / `<sce:on-overflow>` / `<sce:ordering>` /
/// `<sce:concurrency>` lines without rebuilding the full XML.
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

/// Compile a codec + BC pair through `compile_scxml_with_imports`
/// and return the generated Rust source for the BC. Panics on any
/// validator or codegen failure.
fn compile_pair(bc_xml: &str, codec_xml: &str, codec_basename: &str, bc_basename: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let codec = write_doc(dir.path(), codec_basename, codec_xml);
    let bc = write_doc(dir.path(), bc_basename, bc_xml);
    let outputs = compile_scxml_with_imports(
        &[],
        &[codec.as_path(), bc.as_path()],
        &template_dir(),
        Language::Rust,
        &ForgeCompileOptions::default(),
        None,
    )
    .expect("orchestrator codegen succeeds");

    // Find the BC's emission by filename.
    let bc_output = outputs
        .iter()
        .find(|(name, _)| name == bc_basename)
        .unwrap_or_else(|| {
            panic!(
                "BC output not present; got entries: {:?}",
                outputs.iter().map(|(n, _)| n).collect::<Vec<_>>()
            )
        });
    // GeneratedOutput.files is a `Vec<(String, String)>`; the BC
    // template emits a single file. Pull its body verbatim.
    let body = bc_output
        .1
        .files
        .iter()
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n\n");
    body
}

fn assert_parses(label: &str, code: &str) {
    if let Err(e) = syn::parse_file(code) {
        panic!("{label}: emit does not parse as Rust: {e}\n--- code ---\n{code}");
    }
}

// ─── 1. happy_compile_const_no_index_by ─────────────────────────────

#[test]
fn happy_compile_const_no_index_by() {
    // CompileConst capacity + no index-by + default policies.
    // Verifies the baseline emit shape: Handle struct, OverflowError
    // struct, insert/remove/get/iter/len/capacity methods, the
    // element-type import, and the bitmap/generation arrays.
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair(
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert_parses("happy_compile_const_no_index_by", &code);

    // Element-type import (spec lines 2566-2567).
    assert!(
        code.contains("use super::subscription_entry::SubscriptionEntry;"),
        "expected `use super::subscription_entry::SubscriptionEntry;` in emit; got:\n{code}"
    );

    // CAPACITY const carries the schema value.
    assert!(
        code.contains("pub const CAPACITY: usize = 8;"),
        "expected `pub const CAPACITY: usize = 8;` in emit"
    );

    // Handle newtype + 16/16 bit allocation.
    assert!(code.contains("pub struct LocalSubTableHandle(u32);"));
    assert!(code.contains("const SLOT_BITS: u32 = 16;"));
    assert!(code.contains("const GEN_BITS: u32 = 16;"));

    // Operations contract (spec lines 2609-2619).
    assert!(code.contains(
        "pub fn insert(&mut self, elem: SubscriptionEntry) -> \
         Result<LocalSubTableHandle, LocalSubTableOverflowError>"
    ));
    assert!(code.contains("pub fn remove(&mut self, handle: LocalSubTableHandle) -> bool"));
    assert!(code
        .contains("pub fn get(&self, handle: LocalSubTableHandle) -> Option<&SubscriptionEntry>"));
    assert!(code.contains("pub fn iter(&self) -> impl Iterator<Item = &SubscriptionEntry>"));
    assert!(code.contains("pub fn len(&self) -> usize"));
    assert!(code.contains("pub const fn capacity() -> usize"));

    // No find_by_index method (the comment in the file header
    // mentions the method by name as part of the operations contract
    // docstring — only the `pub fn find_by_index` definition needs
    // to be absent under no index-by).
    assert!(
        !code.contains("pub fn find_by_index"),
        "no <sce:index-by> means no `pub fn find_by_index` method emit"
    );

    // cfg-conditional backing storage (spec line 2571).
    assert!(code.contains("#[cfg(not(feature = \"no_std\"))]"));
    assert!(code.contains("#[cfg(feature = \"no_std\")]"));
    assert!(code.contains("::std::vec::Vec<Option<SubscriptionEntry>>"));
    // Reached through the runtime's re-export, not as `::heapless`. This
    // alias is private and no public signature exposes a heapless type, so
    // naming the crate directly would oblige every consumer to declare a
    // dependency for code they never wrote. Asserted in the full form
    // because the bare `::heapless::Vec<…>` spelling is a substring of the
    // re-exported one — a check for the short form passes either way and
    // would not notice the regression.
    assert!(
        code.contains("::sce_forge_runtime::heapless::Vec<Option<SubscriptionEntry>, CAPACITY>")
    );

    // Generation array + occupancy bitmap.
    assert!(code.contains("generation: [u32; CAPACITY]"));
    assert!(code.contains("bitmap: [u32; BITMAP_WORDS]"));
    assert!(code.contains("const BITMAP_WORDS: usize = (CAPACITY + 31) / 32;"));
}

// ─── 2. happy_compile_const_with_index_by ─────────────────────────────

#[test]
fn happy_compile_const_with_index_by() {
    // CompileConst + <sce:index-by field="key_expr_id"/>. Verifies
    // that find_by_index emits with the resolved Rust type (`u32`
    // from the codec's `key_expr_id: uint32` field).
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        16,
        "\n  <sce:index-by field=\"key_expr_id\"/>",
    );
    let code = compile_pair(
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert_parses("happy_compile_const_with_index_by", &code);

    // find_by_index emits with `&u32` (spec line 2615 + codec field
    // type resolved by orchestrator).
    assert!(
        code.contains("pub fn find_by_index(&self, key: &u32) -> Option<LocalSubTableHandle>"),
        "expected typed find_by_index over &u32; got:\n{code}"
    );

    // The field name appears in the comparison body.
    assert!(
        code.contains("&elem.key_expr_id == key"),
        "expected `&elem.key_expr_id == key` comparison body"
    );
}

// ─── 3. capacity_const_matches_schema ─────────────────────────────────

#[test]
fn capacity_const_matches_schema() {
    // Two different `<sce:capacity const="N">` values must lower to
    // distinct `CAPACITY: usize = N` literals in two different emits.
    for n in [1u32, 7, 64, 1024] {
        let codec = codec_doc("subscription_entry");
        let bc = bc_doc("local_sub_table", "subscription_entry", n, "");
        let code = compile_pair(
            &bc,
            &codec,
            "subscription_entry.scxml",
            "local_sub_table.scxml",
        );
        assert_parses(&format!("capacity_const_matches_schema_n={n}"), &code);
        let needle = format!("pub const CAPACITY: usize = {n};");
        assert!(
            code.contains(&needle),
            "expected `{needle}` for capacity={n}; got:\n{code}"
        );
    }
}

// ─── 4. overflow_reject_emits_err_return ──────────────────────────────

#[test]
fn overflow_reject_emits_err_return() {
    // `<sce:on-overflow>reject</sce:on-overflow>`: insert's full-table
    // arm must end in `Err(...OverflowError)` — no eviction code.
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>reject</sce:on-overflow>",
    );
    let code = compile_pair(
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert_parses("overflow_reject_emits_err_return", &code);
    assert!(
        code.contains("Err(LocalSubTableOverflowError)"),
        "reject policy must emit `Err(...OverflowError)`; got:\n{code}"
    );
    // No eviction path emitted for reject policy.
    assert!(
        !code.contains("oldest_occupied_slot"),
        "reject policy must NOT emit `oldest_occupied_slot`"
    );
}

// ─── 5. overflow_oldest_wins_evicts ────────────────────────────────────

#[test]
fn overflow_oldest_wins_evicts() {
    // `<sce:on-overflow>oldest-wins</sce:on-overflow>` combined with
    // the default `<sce:ordering>insertion`. insert's full-table arm
    // must call `oldest_occupied_slot` and increment the slot's
    // generation counter, returning `Ok`.
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc(
        "local_sub_table",
        "subscription_entry",
        4,
        "\n  <sce:on-overflow>oldest-wins</sce:on-overflow>",
    );
    let code = compile_pair(
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert_parses("overflow_oldest_wins_evicts", &code);
    assert!(
        code.contains("fn oldest_occupied_slot"),
        "oldest-wins must emit `oldest_occupied_slot` helper; got:\n{code}"
    );
    assert!(
        code.contains("self.generation[slot] = self.generation[slot].wrapping_add(1) & GEN_MASK;"),
        "oldest-wins must increment the slot's generation counter"
    );
    // No `Err(OverflowError)` arm in insert — the table never
    // refuses an insert under oldest-wins.
    let insert_start = code
        .find("pub fn insert")
        .expect("insert method must exist");
    let insert_end = insert_start
        + code[insert_start..]
            .find("    pub fn remove")
            .expect("remove method must follow insert");
    let insert_body = &code[insert_start..insert_end];
    assert!(
        !insert_body.contains("Err(LocalSubTableOverflowError)"),
        "oldest-wins insert() body must not return Err"
    );
}

// ─── 6. handle_packed_layout ────────────────────────────────────────

#[test]
fn handle_packed_layout() {
    // Locked layout: SLOT_BITS=16, GEN_BITS=16; the Handle is a
    // tuple newtype over u32. Both must
    // appear verbatim in the emit.
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair(
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert_parses("handle_packed_layout", &code);
    assert!(code.contains("pub struct LocalSubTableHandle(u32);"));
    assert!(code.contains("const SLOT_BITS: u32 = 16;"));
    assert!(code.contains("const GEN_BITS: u32 = 16;"));
    assert!(code.contains("const SLOT_MASK: u32 = (1u32 << SLOT_BITS) - 1;"));
    assert!(code.contains("const GEN_MASK: u32 = (1u32 << GEN_BITS) - 1;"));
    // Slot + generation accessors.
    assert!(code.contains("pub const fn slot(self) -> u32"));
    assert!(code.contains("pub const fn generation(self) -> u32"));
}

// ─── 7. use_after_remove_branches_present ──────────────────────────────

#[test]
fn use_after_remove_branches_present() {
    // The generation check must appear in both `remove` and `get`
    // (spec lines 2613-2614 + 2621-2622). This is the use-after-
    // remove invariant — a stale handle whose generation no longer
    // matches the slot's current generation must return false /
    // None.
    let codec = codec_doc("subscription_entry");
    let bc = bc_doc("local_sub_table", "subscription_entry", 8, "");
    let code = compile_pair(
        &bc,
        &codec,
        "subscription_entry.scxml",
        "local_sub_table.scxml",
    );

    assert_parses("use_after_remove_branches_present", &code);
    let needle = "if self.generation[slot] != handle.generation() {";
    let occurrences = code.matches(needle).count();
    assert!(
        occurrences >= 2,
        "expected generation check in both remove() and get() (>= 2 occurrences); \
         found {occurrences}\n--- code ---\n{code}"
    );
    // remove increments the slot's generation counter on free.
    assert!(
        code.contains("self.generation[slot] = self.generation[slot].wrapping_add(1) & GEN_MASK;")
    );
}

// ─── 8. deploy_key_resolves_into_emit ───────────────────────────────────

#[test]
fn deploy_key_resolves_into_emit() {
    // `<sce:capacity source="deploy" key="machines.<m>.limits.<k>"/>`
    // routes through compile_forge_with_deploy. γ1 validates the
    // limit exists; γ2 populates the resolved value into options;
    // the render layer lowers it to `CAPACITY: usize = <value>`.
    let bc = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>subscription_entry</sce:element-type>
  <sce:capacity source="deploy" key="machines.mcu_node.limits.local_subscriptions"/>
</scxml>"##;
    let deploy_yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
          local_subscriptions: 42
"##;
    let deploy = parse_deploy_str(deploy_yaml).expect("deploy parses");
    let out = compile_forge_with_deploy(
        bc,
        DocumentLabel::symmetric("local_sub_table"),
        Language::Rust,
        Some(&deploy),
        Some("mcu_node"),
    );
    let output = match out {
        Ok(o) => o,
        Err(e) => panic!("deploy-key BC must emit cleanly; got: {:?}", e.error),
    };
    let body = output
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n");
    assert_parses("deploy_key_resolves_into_emit", &body);
    assert!(
        body.contains("pub const CAPACITY: usize = 42;"),
        "expected resolved capacity `= 42` in emit; got:\n{body}"
    );
}

// ─── 9. element_type_procedure_emits ────────────────────────────────────

#[test]
fn element_type_procedure_emits() {
    // `<sce:element-type>` resolving to a procedure (not a codec)
    // still emits the `use super::<snake>::<Pascal>;` import; the
    // index-by field type comes from the procedure's inputs /
    // internals slice.
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
    let outputs = compile_scxml_with_imports(
        &[],
        &[proc_path.as_path(), bc_path.as_path()],
        &template_dir(),
        Language::Rust,
        &ForgeCompileOptions::default(),
        None,
    )
    .expect("orchestrator codegen succeeds");
    let bc_output = outputs
        .iter()
        .find(|(name, _)| name == "reassembly_table.scxml")
        .expect("BC output present");
    let body = bc_output
        .1
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n\n");

    assert_parses("element_type_procedure_emits", &body);
    assert!(body.contains("use super::frame_record::FrameRecord;"));
    // checksum is `uint64` → emitted as `&u64`.
    assert!(
        body.contains("pub fn find_by_index(&self, key: &u64) -> Option<ReassemblyTableHandle>"),
        "expected typed find_by_index over &u64 (from procedure's internal field); got:\n{body}"
    );
}

// ─── Element storage profile ────────────────────────────────────────
//
// A bounded collection promises fixed capacity and no allocation. That
// promise has to reach the *elements*, not just the slot table: storing
// default-profile mirrors would leave every element's text, bytes and nested
// lists on the heap in an `alloc` build, so the container would allocate per
// insert while advertising that it does not. The emit therefore pins the
// element's owned mirror to the non-allocating storage profile.
//
// Pinning that profile is also what admits a *composite* element — one whose
// fields include embedded bodies or bounded lists — because the profile
// resolves those containers inline too. Before the storage parameter existed
// such a mirror was `#[cfg(feature = "alloc")]` and had no no-alloc form to
// store at all.

/// A body codec carrying a bounded string, so codecs that embed or list it
/// are borrowed and get an owned mirror.
fn borrowed_body_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <datamodel>
    <sce:field id="hop_len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="hop" sce:type="string" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="hop_len" sce:max-size="16"/>
  </datamodel>
</scxml>"##
    )
}

/// A body codec with fixed-width fields only — no owned mirror is emitted for
/// it, nor for a codec that merely lists it.
fn fixed_body_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <datamodel>
    <sce:field id="metric" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"##
    )
}

/// An element codec whose only complex field is a bounded list of `body`.
fn list_element_doc(name: &str, body: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <sce:import src="{body}.scxml" kind="codec" as="{body}"/>
  <datamodel>
    <sce:field id="hop_count" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:repeat id="hops" type="{body}" sce:byte="1"
                count="hop_count" max-count="4"/>
  </datamodel>
</scxml>"##
    )
}

/// Compile an element codec, its body codec and a BC over the element,
/// returning the BC's generated Rust source.
fn compile_triple(body_xml: &str, element_xml: &str, bc_xml: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let body = write_doc(dir.path(), "route_body.scxml", body_xml);
    let element = write_doc(dir.path(), "route_entry.scxml", element_xml);
    let bc = write_doc(dir.path(), "route_table.scxml", bc_xml);
    let outputs = compile_scxml_with_imports(
        &[],
        &[body.as_path(), element.as_path(), bc.as_path()],
        &template_dir(),
        Language::Rust,
        &ForgeCompileOptions::default(),
        None,
    )
    .expect("orchestrator codegen succeeds");
    outputs
        .iter()
        .find(|(name, _)| name == "route_table.scxml")
        .expect("BC output present")
        .1
        .files
        .iter()
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A composite element — its fields include a bounded list of a borrowed body
/// — is stored as its owned mirror at the non-allocating profile. The `use`
/// brings in the bare name (a path cannot carry generic arguments) while every
/// storage site names the profile.
#[test]
fn composite_element_is_stored_as_the_inline_owned_mirror() {
    let code = compile_triple(
        &borrowed_body_doc("route_body"),
        &list_element_doc("route_entry", "route_body"),
        &bc_doc("route_table", "route_entry", 4, ""),
    );
    assert_parses("composite element BC", &code);
    assert!(
        code.contains("use super::route_entry::RouteEntryOwned;"),
        "the mirror must be imported by bare name; got:\n{code}"
    );
    assert!(
        code.contains("RouteEntryOwned<::sce_forge_runtime::codec::Inline>"),
        "the element must be stored at the non-allocating profile; got:\n{code}"
    );
    assert!(
        !code.contains("elem: RouteEntryOwned,"),
        "no storage site may fall back to the build's default profile;\n{code}"
    );
}

/// An element whose list holds *fixed-width* bodies is not borrowed, so no
/// owned mirror is emitted for it and the element is stored directly.
///
/// This is the case a field-list guess gets wrong: a `<sce:repeat>` field
/// carries a `bytes` sentinel (its real type is the body codec), so reading
/// the element's field types would classify this as borrowed and name a mirror
/// that is never emitted. The storage decision reads the same predicate the
/// codec emit uses instead.
#[test]
fn list_of_fixed_width_bodies_is_stored_directly() {
    let code = compile_triple(
        &fixed_body_doc("route_body"),
        &list_element_doc("route_entry", "route_body"),
        &bc_doc("route_table", "route_entry", 4, ""),
    );
    assert_parses("fixed-body list BC", &code);
    assert!(
        code.contains("use super::route_entry::RouteEntry;"),
        "a lifetime-free element is its own owned form; got:\n{code}"
    );
    assert!(
        !code.contains("RouteEntryOwned"),
        "no owned mirror is emitted for a non-borrowed element, so none may \
         be referenced; got:\n{code}"
    );
}

/// The same pinning applies to the flat bounded-string element the earlier
/// bounded-collection work shipped: it was already stored as a mirror, but at
/// the build's default profile, which allocates per element under `alloc`.
#[test]
fn bounded_string_element_is_stored_at_the_inline_profile() {
    let codec = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="pattern_entry" version="1.0">
  <datamodel>
    <sce:field id="pattern_len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="pattern" sce:type="string" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="pattern_len" sce:max-size="32"/>
  </datamodel>
</scxml>"##;
    let code = compile_pair(
        &bc_doc("pattern_table", "pattern_entry", 8, ""),
        codec,
        "pattern_entry.scxml",
        "pattern_table.scxml",
    );
    assert_parses("bounded-string element BC", &code);
    assert!(
        code.contains("PatternEntryOwned<::sce_forge_runtime::codec::Inline>"),
        "a bounded collection must not hold heap-backed elements; got:\n{code}"
    );
}

/// The consumer claim, on a real compiler: a bounded collection of *composite*
/// elements builds with no allocator in reach.
///
/// Parsing the emit cannot show this — the question is whether the element's
/// nested list and bounded string resolve to inline containers once
/// monomorphised. The crate is built with default features, so neither the
/// runtime nor the generated code has `alloc`, and a single reachable `Vec` /
/// `String` is a hard error. Before the storage profile existed the mirror for
/// such an element was `#[cfg(feature = "alloc")]`, so this crate had no
/// element type to name at all.
#[test]
fn composite_element_bounded_collection_builds_without_an_allocator() {
    let dir = tempdir().expect("tempdir");
    let body = write_doc(
        dir.path(),
        "route_body.scxml",
        &borrowed_body_doc("route_body"),
    );
    let element = write_doc(
        dir.path(),
        "route_entry.scxml",
        &list_element_doc("route_entry", "route_body"),
    );
    let bc = write_doc(
        dir.path(),
        "route_table.scxml",
        &bc_doc("route_table", "route_entry", 4, ""),
    );
    let outputs = compile_scxml_with_imports(
        &[],
        &[body.as_path(), element.as_path(), bc.as_path()],
        &template_dir(),
        Language::Rust,
        &ForgeCompileOptions::default(),
        None,
    )
    .expect("orchestrator codegen succeeds");

    let crate_dir = dir.path().join("bc_no_alloc");
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let mut modules = Vec::new();
    for (_doc, generated) in &outputs {
        for (filename, content) in &generated.files {
            fs::write(src_dir.join(filename), content)
                .unwrap_or_else(|e| panic!("write {filename}: {e}"));
            if let Some(stem) = filename.strip_suffix(".rs") {
                if !stem.ends_with("_test") {
                    modules.push(stem.to_string());
                }
            }
        }
    }
    // Exercise the container so the element type is monomorphised rather than
    // merely declared: inserting and reading back is where a stray heap
    // container in the element would surface.
    modules.push("probe".to_string());
    fs::write(
        src_dir.join("probe.rs"),
        "use crate::route_table::RouteTable;\n\
         use crate::route_entry::RouteEntryOwned;\n\
         use sce_forge_runtime::codec::Inline;\n\
         \n\
         pub fn insert_and_read(\n    \
             table: &mut RouteTable,\n    \
             entry: RouteEntryOwned<Inline>,\n\
         ) -> Option<u8> {\n    \
             let handle = table.insert(entry).ok()?;\n    \
             table.get(handle).map(|e| e.hop_count)\n\
         }\n",
    )
    .expect("write probe.rs");
    let lib_rs: String = std::iter::once("#![no_std]\n".to_string())
        .chain(modules.iter().map(|m| format!("pub mod {m};\n")))
        .collect();
    fs::write(src_dir.join("lib.rs"), lib_rs).expect("write lib.rs");

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let runtime_path = repo_root.join("backends/rust/forge-runtime");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\n\
             name = \"bc_no_alloc_gate\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             \n\
             [lib]\n\
             path = \"src/lib.rs\"\n\
             \n\
             [features]\n\
             default = [\"no_std\"]\n\
             no_std = []\n\
             \n\
             # One entry, deliberately. The emitted collection stores its\n\
             # slots in a heapless vector, and this crate declaring heapless\n\
             # too is what kept the template free to name `::heapless`\n\
             # directly — a dependency no consumer manifest was ever told\n\
             # about. With the backing reached through the runtime's\n\
             # re-export, a second line here would restore that blind spot.\n\
             [dependencies]\n\
             sce-forge-runtime = {{ path = {runtime:?} }}\n\
             \n\
             [workspace]\n",
            runtime = runtime_path.to_string_lossy(),
        ),
    )
    .expect("write Cargo.toml");

    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", dir.path().join("target"))
        .output()
        .expect("run cargo build");
    assert!(
        output.status.success(),
        "no-alloc build of a bounded collection over a composite element failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// The probe crate used to re-derive the workspace's pinned `heapless`
// version so its own dependency line could not drift onto a different one.
// It no longer has that line: the emitted backing reaches heapless through
// `sce_forge_runtime::heapless`, so the single pin the runtime carries is
// the only one in play and there is nothing left to keep in step.
