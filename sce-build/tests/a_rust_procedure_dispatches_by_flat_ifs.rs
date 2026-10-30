// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Rust a procedure generates dispatches its entry actions, its
//! transitions and its assignments as flat `if`s, names the boxed closures it
//! holds, and has `Default`.
//!
//! The shape it replaced was a `match` over the state with an `if` per
//! transition inside each arm. Whenever a state held one entry action or one
//! transition, that was a `match` of one arm or of none, or an arm whose whole
//! body was one `if`, and clippy refuses each under `-D warnings`
//! (`single_match`, `match_single_binding`, `collapsible_match`,
//! `collapsible_if`); it refuses a public type with a `new()` and no `Default`
//! (`new_without_default`) and a `Box<dyn Fn(..) -> ..>` field
//! (`type_complexity`) as well. No workspace lint reached that code: the
//! catalog compiles it only in the conformance crate with its `alloc` feature
//! on. These read the shape off the generated text.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_with_imports, load_forge_source, DocumentLabel, ForgeCompileOptions,
};

fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The Rust a catalog procedure generates, under the label the conformance
/// crate compiles it under.
fn generated(name: &str) -> String {
    let path = resources().join(format!("{name}.scxml"));
    let loaded =
        load_forge_source(&path, &[]).unwrap_or_else(|e| panic!("expand {}: {e}", path.display()));
    let output = compile_forge_with_imports(
        loaded.text(),
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resources(),
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("generate {name}: {e}"));
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Entry actions, transitions and assignments of a procedure with sends,
/// guards and an assignment, each a flat `if`; the guard is parenthesised whole.
#[test]
fn a_procedure_with_guards_and_sends_dispatches_by_flat_ifs() {
    let code = generated("procedure_security_access");
    for line in [
        "        if state == State::SendTesterPresent {",
        "        if state == State::Done {",
        "        if state == State::SendTesterPresent && event == Event::Ok {",
        "        if state == State::Retry && event == Event::None && (self.retry_count < self.max_retries) {",
        "        if source == State::Retry && tr_index == 0 {",
    ] {
        assert!(
            code.lines().any(|l| l == line),
            "the dispatch line `{line}` is missing:\n{code}"
        );
    }
    // The fallback arm of a `match` over the state is the shape that was a
    // `match` of one arm or of none.
    assert!(
        !code.contains("_ => {}"),
        "a dispatch is still a `match` with an empty fallback arm:\n{code}"
    );
}

/// The boxed closures the policy holds are named, and the policy has `Default`.
#[test]
fn a_procedure_names_its_boxed_closures_and_has_default() {
    let code = generated("procedure_security_access");
    for line in [
        "type BoxedServiceHandler = Box<dyn Fn(&ProcedureServiceRequest) -> ProcedureServiceResponse>;",
        "type ComputeKeyHelper = Box<dyn Fn(&[u8]) -> Vec<u8>>;",
        "    service_handler: Option<BoxedServiceHandler>,",
        "    compute_key: ComputeKeyHelper,",
        "impl Default for ProcedureSecurityAccess {",
    ] {
        assert!(
            code.lines().any(|l| l == line),
            "the line `{line}` is missing:\n{code}"
        );
    }
}

/// A procedure with no entry action reads the state it is asked about once,
/// and dispatches nothing on it.
#[test]
fn a_procedure_with_no_entry_action_reads_its_state_once() {
    let code = generated("procedure_linear");
    assert!(
        code.lines()
            .any(|l| l == "        let _ = state; // no state has an entry action"),
        "the entry dispatch of a procedure with none is not empty:\n{code}"
    );
    assert!(
        code.lines()
            .any(|l| l == "        if state == State::StageC && event == Event::None {"),
        "an eventless transition is not one flat `if`:\n{code}"
    );
    assert!(
        !code.contains("_ => {}"),
        "a dispatch is still a `match` with an empty fallback arm:\n{code}"
    );
}
