// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The last bullet of docs/adr/0087, "Landing order" item 8: a field of a variable of
//! `optional<record:R>` whose record holds a composition is read where a test proves
//! the variable present, in a guard and in an assignment, on every generated lane and
//! the Interpreter, as a field of an optional record of entries is read
//! (docs/adr/0055, step 9): a text, a number, and the length of the list it holds.
//!
//! Held to `sce-build/tests/fixtures/static_datamodel/static_lift_list.scxml`, whose
//! `answer.read` is guarded by `answer.value.title` and `answer.value.priority`, and
//! whose `text.given` takes `answer.value.priority` and `len(answer.value.items)` into
//! variables; `scenarios/static_lift_list.json` replays it on each lane. This file
//! holds the text the generator writes for those reads.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

/// The fixture, where it lies beside the schemas it imports.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_datamodel/static_lift_list.scxml")
}

/// Every file `language` generates of the fixture, one after the other.
fn machine(language: Language) -> String {
    let output = compile_scxml_lang_typed(
        fixture().to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} lowers static_lift_list: {e:?}"));
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `code` holds each of `reads`.
fn holds(language: &str, code: &str, reads: &[&str]) {
    for read in reads {
        assert!(
            code.contains(read),
            "{language}: expected `{read}` in:\n{code}"
        );
    }
}

/// Pinned by `rust_optional_form`'s proven read of a record
/// (`sce-build/src/forge/expr.rs`), which reads it in place through `proven_ref`.
#[test]
fn rust_reads_the_fields_of_the_present_record_in_place() {
    holds(
        "Rust",
        &machine(Language::Rust),
        &[
            "sce_forge_runtime::optional::proven_ref(&self.answer).title",
            "sce_forge_runtime::optional::proven_ref(&self.answer).priority",
            "sce_forge_runtime::optional::proven_ref(&self.answer).items",
        ],
    );
}

/// Pinned by `kotlin_optional_form`'s proven read (`KOTLIN_PROVEN`).
#[test]
fn kotlin_reads_the_fields_of_the_present_record() {
    holds(
        "Kotlin",
        &machine(Language::Kotlin),
        &[
            "com.sce.forge.runtime.SceOptional.proven(vAnswer).title",
            "com.sce.forge.runtime.SceOptional.proven(vAnswer).priority",
            "com.sce.forge.runtime.SceOptional.proven(vAnswer).items",
        ],
    );
}

/// Pinned by `go_optional`'s proven read (`x.Proven()`).
#[test]
fn go_reads_the_fields_of_the_present_record() {
    holds(
        "Go",
        &machine(Language::Go),
        &[
            "p.vAnswer.IsPresent()",
            "p.vAnswer.Proven().title",
            "p.vAnswer.Proven().priority",
            "p.vAnswer.Proven().items",
        ],
    );
}

/// Pinned by `python_optional`'s proven read, which is the value itself.
#[test]
fn python_reads_the_fields_of_the_present_record() {
    holds(
        "Python",
        &machine(Language::Python),
        &[
            "self.v_answer is not None",
            "self.v_answer.title",
            "self.v_answer.priority",
            "self.v_answer.items",
        ],
    );
}

/// Pinned by `cpp_optional`'s proven read (`(*x)`).
#[test]
fn cpp_reads_the_fields_of_the_present_record() {
    holds(
        "C++",
        &machine(Language::Cpp),
        &[
            "v_answer.has_value()",
            "(*v_answer).title",
            "(*v_answer).priority",
            "(*v_answer).items",
        ],
    );
}

/// Pinned by `swift_optional_form`'s proven read (`SceOptional.proven`).
#[test]
fn swift_reads_the_fields_of_the_present_record() {
    holds(
        "Swift",
        &machine(Language::Swift),
        &[
            "SceOptional.proven(self.vAnswer).title",
            "SceOptional.proven(self.vAnswer).priority",
            "SceOptional.proven(self.vAnswer).items",
        ],
    );
}

/// Pinned by the Interpreter's proven read (`emit_js`), which reads the plain object
/// the variable holds.
#[test]
fn the_interpreter_reads_the_fields_of_the_present_record() {
    let path = fixture();
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers static_lift_list: {e:?}"));
    holds(
        "Interpreter",
        &lowered,
        &["answer.title", "answer.priority", "answer.items"],
    );
}
