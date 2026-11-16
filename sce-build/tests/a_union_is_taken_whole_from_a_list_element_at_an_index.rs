// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A union variable takes a list's element at an index whole on every lane
//! (docs/adr/0099, section 5.3; docs/adr/0092, section 6.1 item 5), held to
//! the `shapes.picked` transition of
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The element is read by the lane's checked read of an index (SCE_FORGE.md
//!   §3.4.1), which fails `out-of-range` for an index outside the list and
//!   answers no union in its place: a thrown failure (Rust, Swift, Kotlin,
//!   Python) or a recorded one beside the language's own default of the type
//!   (C++, Go, C11; docs/adr/0099, section 2.2).
//! * The copy is written only after the read completed, and a failed read
//!   raises `error.execution` and ends the block (§scxml-4.9).
//! * A union that may be absent takes the same element as its present value.
//!
//! What the generated machines do at run time is held by the scenario
//! `static_union.json`, replayed on every lane.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel/static_union.scxml")
}

/// Every file `language` generates for the fixture, one after the other.
fn machine(language: Language) -> String {
    compile_scxml_lang_typed(
        fixture().to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} takes a union whole from a list's element: {e:?}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// What every lane raises in place of the failed copy.
const RAISED: &[&str] = &[
    "<assign location='picked'>: an integer operation overflowed or failed",
    "<assign location='picked_maybe'>: an integer operation overflowed or failed",
];

/// Pinned by `RustTarget::element_at` (`sce-build/src/forge/static_lowering.rs`):
/// the element owns its parts, so it is read by reference and cloned out of the
/// list, and the runtime's `?` leaves the statement before the write.
#[test]
fn rust_reads_the_element_by_reference_and_clones_it_out() {
    holds(
        &machine(Language::Rust),
        &[
            "self.picked = sce_forge_runtime::algorithm::at_ref(&ev.many, ev.at)?.clone();",
            "self.picked_maybe = Some(sce_forge_runtime::algorithm::at_ref(&ev.many, ev.at)?.clone());",
        ],
    );
}

/// Pinned by `SwiftTarget::receiving_write_of`: the local names the variable's
/// type, and the write is in the `do` after the throwing read.
#[test]
fn swift_holds_the_read_in_a_local_of_the_variables_type() {
    let code = machine(Language::Swift);
    holds(
        &code,
        &[
            "let sceValue: StaticUnionUnionShapeUnion = (try Checked.at(self.pendingShapesPickedPayload.many, \
             self.pendingShapesPickedPayload.at)); self.vPicked = sceValue }",
            "let sceValue: StaticUnionUnionShapeUnion? = (try Checked.at(self.pendingShapesPickedPayload.many, \
             self.pendingShapesPickedPayload.at)); self.vPickedMaybe = sceValue }",
        ],
    );
    holds(&code, RAISED);
}

/// Pinned by `KotlinTarget::record_at`: the runtime's checked read throws
/// before the assignment.
#[test]
fn kotlin_reads_the_element_through_the_checked_read() {
    let code = machine(Language::Kotlin);
    holds(
        &code,
        &[
            "vPicked = com.sce.forge.runtime.SceChecked.at(pendingShapesPickedPayload!!.many, \
             (pendingShapesPickedPayload!!.at).toLong()); false }",
            "vPickedMaybe = com.sce.forge.runtime.SceChecked.at(pendingShapesPickedPayload!!.many, \
             (pendingShapesPickedPayload!!.at).toLong()); false }",
        ],
    );
    holds(&code, RAISED);
}

/// Pinned by `PythonTarget::record_at` and the Python copy of a union.
#[test]
fn python_copies_the_element_the_checked_read_answers() {
    holds(
        &machine(Language::Python),
        &[
            "self.v_picked = sce_algorithm.whole_copy(sce_algorithm.at(self._pending_shapes_picked_payload.many, \
             self._pending_shapes_picked_payload.at))",
            "self.v_picked_maybe = sce_algorithm.whole_copy(sce_algorithm.at(\
             self._pending_shapes_picked_payload.many, self._pending_shapes_picked_payload.at))",
        ],
    );
}

/// Pinned by `GoTarget::record_at` and `GoTarget::receiving_write`: the value
/// is computed into a local and written only when no failure was recorded.
#[test]
fn go_writes_the_element_only_when_no_failure_was_recorded() {
    let code = machine(Language::Go);
    holds(
        &code,
        &[
            "sceValue := scealgorithm.At(&sceFailure, p.pendingShapesPickedPayload.many[:], \
             int64(p.pendingShapesPickedPayload.at)); if sceFailure.Failed() {",
            "sceValue := optional.Some(scealgorithm.At(&sceFailure, p.pendingShapesPickedPayload.many[:], \
             int64(p.pendingShapesPickedPayload.at))); if sceFailure.Failed() {",
            "return true }; p.vPicked = sceValue; return false }()",
        ],
    );
    holds(&code, RAISED);
}

/// Pinned by `CppTarget::record_at` and `CppTarget::receiving_write`.
#[test]
fn cpp_writes_the_element_only_when_no_failure_was_recorded() {
    let code = machine(Language::Cpp);
    holds(
        &code,
        &["auto sce_value = SCE::Forge::Checked::at(sce_failure_, pendingShapesPickedPayload_.many,"],
    );
    holds(&code, RAISED);
}

/// Pinned by `CTarget::record_at` and `CTarget::receiving_write_of`: the local
/// names the variable's type, the language's own `{0}` stands beside a failed
/// read and is never written (docs/adr/0099, section 2.2).
#[test]
fn c11_writes_the_element_only_when_no_failure_was_recorded() {
    let code = machine(Language::C11);
    holds(
        &code,
        &[
            "static_union_union_union_shape_t sce_value = ((sce_forge_checked_index_from_i(&sce_failure_, \
             (int64_t)(sm->pending_payload.as.shapes_picked.at), (sm->pending_payload.as.shapes_picked.many).len)",
            "sce_optional_record_static_union_union_union_shape_t sce_value = \
             sce_optional_record_static_union_union_union_shape_some(",
            "sm->policy.v_picked = sce_value;",
            "sm->policy.v_picked_maybe = sce_value;",
        ],
    );
    holds(&code, RAISED);
}

// ── A list itself reached through an index (docs/adr/0099, "Still open
//    (2026-11-16)" item 3): the `shapes.grouped` and `shapes.rowed` transitions,
//    the list a group at an index holds and a row at an index of a list of rows.
//    Each index is read by the lane's checked read in turn, the outer read nested
//    in the inner one as written, so an index outside either list fails the copy.

/// Pinned by `RustTarget::element_within`: the outer element is a reference, a
/// list of lists read through it as it is and a field's list through a borrow.
#[test]
fn rust_reads_each_index_in_turn_through_the_outer_reference() {
    holds(
        &machine(Language::Rust),
        &[
            "self.picked = sce_forge_runtime::algorithm::at_ref(&sce_forge_runtime::algorithm::at_ref(\
             &ev.groups, ev.at)?.shapes, ev.inner)?.clone();",
            "self.picked = sce_forge_runtime::algorithm::at_ref(sce_forge_runtime::algorithm::at_ref(\
             &ev.rows, ev.at)?, ev.inner)?.clone();",
        ],
    );
}

/// Pinned by `StaticTarget::element_within`'s nesting and `SwiftTarget::record_at`.
#[test]
fn swift_reads_each_index_in_turn_each_read_marked() {
    holds(
        &machine(Language::Swift),
        &[
            "(try Checked.at((try Checked.at(self.pendingShapesGroupedPayload.groups, \
             self.pendingShapesGroupedPayload.at)).shapes, self.pendingShapesGroupedPayload.inner))",
            "(try Checked.at((try Checked.at(self.pendingShapesRowedPayload.rows, \
             self.pendingShapesRowedPayload.at)), self.pendingShapesRowedPayload.inner))",
        ],
    );
}

/// Pinned by `StaticTarget::element_within`'s nesting and `KotlinTarget::record_at`.
#[test]
fn kotlin_reads_each_index_in_turn_through_the_checked_read() {
    holds(
        &machine(Language::Kotlin),
        &[
            "com.sce.forge.runtime.SceChecked.at(com.sce.forge.runtime.SceChecked.at(\
             pendingShapesGroupedPayload!!.groups, (pendingShapesGroupedPayload!!.at).toLong()).shapes, \
             (pendingShapesGroupedPayload!!.inner).toLong())",
            "com.sce.forge.runtime.SceChecked.at(com.sce.forge.runtime.SceChecked.at(\
             pendingShapesRowedPayload!!.rows, (pendingShapesRowedPayload!!.at).toLong()), \
             (pendingShapesRowedPayload!!.inner).toLong())",
        ],
    );
}

/// Pinned by `StaticTarget::element_within`'s nesting and `PythonTarget::record_at`.
#[test]
fn python_reads_each_index_in_turn_through_the_checked_read() {
    holds(
        &machine(Language::Python),
        &[
            "sce_algorithm.at(sce_algorithm.at(self._pending_shapes_grouped_payload.groups, \
             self._pending_shapes_grouped_payload.at).shapes, self._pending_shapes_grouped_payload.inner)",
            "sce_algorithm.at(sce_algorithm.at(self._pending_shapes_rowed_payload.rows, \
             self._pending_shapes_rowed_payload.at), self._pending_shapes_rowed_payload.inner)",
        ],
    );
}

/// Pinned by `StaticTarget::element_within`'s nesting and `GoTarget::record_at`: a
/// Go list is a slice, which a slice expression takes whether or not it is
/// addressable, so the inner list is sliced out of the outer read as it stands.
#[test]
fn go_reads_each_index_in_turn_into_the_statements_failure() {
    holds(
        &machine(Language::Go),
        &[
            "scealgorithm.At(&sceFailure, scealgorithm.At(&sceFailure, p.pendingShapesGroupedPayload.groups[:], \
             int64(p.pendingShapesGroupedPayload.at)).shapes[:], int64(p.pendingShapesGroupedPayload.inner))",
            "scealgorithm.At(&sceFailure, scealgorithm.At(&sceFailure, p.pendingShapesRowedPayload.rows[:], \
             int64(p.pendingShapesRowedPayload.at))[:], int64(p.pendingShapesRowedPayload.inner))",
        ],
    );
}

/// Pinned by `StaticTarget::element_within`'s nesting and `CppTarget::record_at`.
#[test]
fn cpp_reads_each_index_in_turn_into_the_statements_failure() {
    holds(
        &machine(Language::Cpp),
        &[
            "SCE::Forge::Checked::at(sce_failure_, SCE::Forge::Checked::at(sce_failure_, \
             pendingShapesGroupedPayload_.groups, ",
            "SCE::Forge::Checked::at(sce_failure_, SCE::Forge::Checked::at(sce_failure_, \
             pendingShapesRowedPayload_.rows, ",
            ").shapes, ",
        ],
    );
}

/// Pinned by `StaticTarget::element_within`'s nesting and `CTarget::record_at`: the
/// inner read's range is the length of the list the outer read answers, which is
/// the empty value of the element where the outer index failed.
#[test]
fn c11_reads_each_index_in_turn_into_the_statements_failure() {
    holds(
        &machine(Language::C11),
        &[
            "(sce_forge_checked_index_from_i(&sce_failure_, \
             (int64_t)(sm->pending_payload.as.shapes_grouped.inner), \
             ((sce_forge_checked_index_from_i(&sce_failure_, \
             (int64_t)(sm->pending_payload.as.shapes_grouped.at), \
             (sm->pending_payload.as.shapes_grouped.groups).len)",
            "(sce_forge_checked_index_from_i(&sce_failure_, \
             (int64_t)(sm->pending_payload.as.shapes_rowed.inner), \
             ((sce_forge_checked_index_from_i(&sce_failure_, \
             (int64_t)(sm->pending_payload.as.shapes_rowed.at), \
             (sm->pending_payload.as.shapes_rowed.rows).len)",
        ],
    );
}
