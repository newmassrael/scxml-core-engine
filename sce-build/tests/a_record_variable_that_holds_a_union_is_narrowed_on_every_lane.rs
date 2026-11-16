// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Step 4 of docs/adr/0099, its record variable (docs/adr/0099, "Decided while landing
//! steps 3 and 4" item 3): `static_union` holds `held`, a variable of
//! `optional<record:Held>` whose record holds a union field, lifted whole from a text
//! (docs/adr/0087, "Landing order" item 8), and narrows the union it holds by its tag
//! where a test proves the record present and the union in its case
//! (docs/adr/0092, section 6.1 item 2): `held.value.shape.width` is read through the
//! present record and the proven case, on every generated lane and the Interpreter.
//!
//! Held to `sce-build/tests/fixtures/static_datamodel/static_union.scxml`, whose
//! scenario (`scenarios/static_union.json`) each lane's driver replays: the lift, the
//! refusal at the union's tag inside the record, the narrowing, and the record saved
//! and read back by its type across the step that restores. This file holds the text
//! the generator writes for the narrowed read.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

/// The fixture, where it lies beside the schemas it imports.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel/static_union.scxml")
}

/// Every file `language` generates of the fixture, one after the other.
fn machine(language: Language) -> String {
    let output = compile_scxml_lang_typed(
        fixture().to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} lowers static_union: {e:?}"));
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `code` holds `read`.
fn holds(language: &str, code: &str, read: &str) {
    assert!(
        code.contains(read),
        "{language}: expected `{read}` in:\n{code}"
    );
}

/// Pinned by `rust_optional_form`'s proven read of a record and `rust_union_member`.
#[test]
fn rust_reads_the_proven_case_through_the_present_record() {
    holds(
        "Rust",
        &machine(Language::Rust),
        "sce_forge_runtime::optional::proven_ref(&self.held).shape.proven_rect().width",
    );
}

/// Pinned by `kotlin_optional_form`'s proven read and `kotlin_union_member`.
#[test]
fn kotlin_reads_the_proven_case_through_the_present_record() {
    holds(
        "Kotlin",
        &machine(Language::Kotlin),
        "com.sce.forge.runtime.SceOptional.proven(vHeld).shape.provenRect().width",
    );
}

/// Pinned by `go_optional`'s proven read and `go_union_member`.
#[test]
fn go_reads_the_proven_case_through_the_present_record() {
    holds(
        "Go",
        &machine(Language::Go),
        "p.vHeld.Proven().shape.ProvenRect().width",
    );
}

/// Pinned by `python_optional`'s proven read and `python_union_member`.
#[test]
fn python_reads_the_proven_case_through_the_present_record() {
    holds(
        "Python",
        &machine(Language::Python),
        "self.v_held.shape.proven_rect().width",
    );
}

/// Pinned by `cpp_optional`'s proven read and `cpp_union_member`.
#[test]
fn cpp_reads_the_proven_case_through_the_present_record() {
    holds(
        "C++",
        &machine(Language::Cpp),
        "(*v_held).shape.provenRect().width",
    );
}

/// Pinned by `swift_optional_form`'s proven read and `swift_union_member`.
#[test]
fn swift_reads_the_proven_case_through_the_present_record() {
    holds(
        "Swift",
        &machine(Language::Swift),
        "SceOptional.proven(self.vHeld).shape.provenRect.width",
    );
}

/// Pinned by `c_present_read` and `c11_union_member`: the present record is the
/// optional struct's `value`, and the case its union's `as` member.
#[test]
fn c11_reads_the_proven_case_through_the_present_record() {
    holds(
        "C11",
        &machine(Language::C11),
        "sm->policy.v_held.value.shape.as.rect.width",
    );
}

/// Pinned by the Interpreter's proven read, which reads the plain object the
/// variable holds, and its union, the wire's own object read by its member's name.
#[test]
fn the_interpreter_reads_the_proven_case_through_the_present_record() {
    let path = fixture();
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers static_union: {e:?}"));
    holds("Interpreter", &lowered, "held.shape.width");
}
