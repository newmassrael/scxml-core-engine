// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Rust machine holds a payload's text to the bound of an algorithm's record
//! where the payload crosses into it (docs/adr/0086, section 7.4): the payload keeps
//! the wire's text until section 4.4 holds it where it is read, on every lane at
//! once, and the algorithm's record holds its bound in its type. The crossing can
//! fail, so the adapter answers a `Result` and the statement that calls it is
//! received as one that can fail, whether or not the algorithm declares
//! `may-fail`; the algorithm's own failure is passed on beside it.
//!
//! The run-time half, a fitting label and one a byte past for each algorithm, is
//! `backends/rust/tests/tests/a_payload_text_is_held_where_it_crosses_into_an_algorithm.rs`.

use std::path::{Path, PathBuf};

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_datamodel/static_payload_text_call.scxml")
}

/// The Rust machine of the fixture.
fn machine() -> String {
    let language = sce_build::generator::Language::Rust;
    let path = fixture();
    compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Rust lowers the fixture: {e:?}"))
    .files
    .into_iter()
    .find(|(name, _)| name == "static_payload_text_call_sm.rs")
    .map(|(_, code)| code)
    .expect("the machine's Rust is generated")
}

/// Pinned by `hands_payload_text` and the `Result` arm of
/// `RustTarget::state_adapter` (`sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_adapter_holds_the_payload_text_and_answers_a_result() {
    let code = machine();
    let held = "::sce_rust_runtime::portable::SceString::<8>::from_text(&p0.label)?";
    assert_eq!(
        code.matches(held).count(),
        2,
        "both adapters hold the payload's label to the record's bound:\n{code}"
    );
    assert_eq!(
        code.matches(
            "-> Result<StaticPayloadTextCallLabelledRecord, sce_forge_runtime::algorithm::AlgorithmError> {"
        )
        .count(),
        2,
        "the adapter of the algorithm that cannot fail answers a Result too:\n{code}"
    );
    // The algorithm that cannot fail is called as it is, and the one that may fail
    // passes its own failure on beside the bound's.
    assert!(code.contains("labelled_echo("), "{code}");
    assert!(
        code.lines().any(|line| line.contains("let r = ")
            && line.contains("labelled_echo(")
            && !line.ends_with("?;")),
        "the call of `Echo` is not received as one that fails:\n{code}"
    );
    assert!(
        code.lines().any(|line| line.contains("let r = ")
            && line.contains("labelled_bump(")
            && line.ends_with("?;")),
        "the call of `Bump` passes its own failure on:\n{code}"
    );
}

/// Pinned by `crossing_calls` and its use in `lowering_ctx`: a call whose adapter
/// holds a payload's text is typed as one that can fail, so the assignment that
/// makes it is received, error.execution raised and nothing written where it fails.
#[test]
fn a_call_that_crosses_a_payload_text_is_received_as_one_that_can_fail() {
    let code = machine();
    assert_eq!(
        code.matches("(&ev)?").count(),
        2,
        "both calls answer through `?`, in the statement's closure:\n{code}"
    );
}
