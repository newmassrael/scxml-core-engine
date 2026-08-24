// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The AI supervision loop is asked the same questions by both engines.
//!
//! `examples/ai_loop/ai_loop.scxml` is deliberately outside the
//! seven-channel stem contract in `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md`:
//! it is a worked example, and the claim its two drivers make is "two
//! engines, one document" rather than "every backend". Both of those files
//! say so in their own header, and `scripts/regen_ai_loop.sh` repeats it.
//!
//! Nothing enforced it. Measured 2026-08-22: the Rust channel asserted 19
//! clauses and the C++ AOT channel 15 — four clauses were the word of one
//! engine only, among them the one this document exists to demonstrate
//! (`§scxml-D-addAncestorStatesToEnter`, whose defect was found on the AOT
//! engines with every W3C fixture green). A clause asserted in one channel
//! is not a claim about the document, and the drift is invisible because
//! both suites stay green while it widens.
//!
//! So this pins the pairing rather than the count: every scenario in either
//! driver has a counterpart in the other. Adding one to either side without
//! the sibling fails here, which is the moment it is cheapest to fix.
//!
//! The two files spell their scenarios in their own language's convention
//! (`answering_a_question_does_not_re_prime_the_session` against
//! `AnsweringAQuestionDoesNotRePrimeTheSession`), so the pairing is by
//! normalised name. That makes the identifier itself the contract: the
//! drivers already name each scenario after the clause it asserts, and
//! matching text is what lets a reader move between the channels.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The document both channels drive. Named here rather than derived, so a
/// regen script or a CMake registration that quietly retargets one channel
/// fails against a written-down answer instead of agreeing with itself.
const AI_LOOP_DOCUMENT: &str = "examples/ai_loop/ai_loop.scxml";

const RUST_DRIVER: &str = "backends/rust/tests/tests/ai_loop.rs";
const CPP_DRIVER: &str = "tests/integration/AiLoopAotTest.cpp";

/// What both channels asserted when the pairing was first enforced. A
/// scanner that stops matching reports two empty sets, and two empty sets
/// pair perfectly — the floor is what makes a broken pattern fail instead
/// of pass. It is a ratchet: raise it when the suite grows, never lower it
/// to accommodate a deletion.
///
/// 19 when the pairing landed (2026-08-22); 24 since 2026-08-23, when
/// counting which of the document's five enumerated outcomes any channel
/// actually reached found `converged` and `failed` at none, `stuck` — one of
/// the two ways into `exhausted` — at none, and both channels driving `judge`
/// without the payload its `cond` reads; 25 when the document's sends became
/// acts a host serves (§scxml-6.2.5) and needed a scenario that RECORDS them,
/// since the silent handler every other scenario registers cannot tell a
/// declared act from a targetless send that lost its `type`; 27 since
/// 2026-08-24, when the resume seam (§scxml-3.2 `enterAt` / `get_state_from_name`)
/// reached the C++ engine and the two scenarios that had been the Rust
/// channel's word alone got their siblings.
const SCENARIO_FLOOR: usize = 27;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

/// Source with whole-line comments removed.
///
/// Both drivers name their sibling's scenarios in prose — that cross
/// reference is the point of the header comments — so a scanner reading
/// comments would pair the two files against their own documentation and
/// report agreement no matter what the code did.
fn without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `answering_a_question…` and `AnsweringAQuestion…` are the same clause
/// spelled in two languages' conventions.
fn normalise(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn scenarios(source: &str, pattern: &str) -> BTreeSet<String> {
    let re = regex::Regex::new(pattern).expect("pattern");
    re.captures_iter(&without_comments(source))
        .map(|c| normalise(&c[1]))
        .collect()
}

fn rust_scenarios() -> BTreeSet<String> {
    scenarios(&read(RUST_DRIVER), r"#\[test\]\s+fn\s+(\w+)")
}

fn cpp_scenarios() -> BTreeSet<String> {
    scenarios(
        &read(CPP_DRIVER),
        r"TEST_F\(\s*AiLoopAotTest\s*,\s*(\w+)\s*\)",
    )
}

#[test]
fn every_ai_loop_clause_is_asserted_by_both_engines() {
    let rust = rust_scenarios();
    let cpp = cpp_scenarios();

    assert!(
        rust.len() >= SCENARIO_FLOOR && cpp.len() >= SCENARIO_FLOOR,
        "expected at least {SCENARIO_FLOOR} scenarios in each channel, found {} in {RUST_DRIVER} \
         and {} in {CPP_DRIVER}. Either a driver lost scenarios, or the pattern reading it stopped \
         matching — and two sets that pair because both are empty prove nothing",
        rust.len(),
        cpp.len(),
    );

    let rust_only: Vec<&String> = rust.difference(&cpp).collect();
    assert!(
        rust_only.is_empty(),
        "{rust_only:?} are asserted only in {RUST_DRIVER}. {AI_LOOP_DOCUMENT} is driven by two \
         engines precisely so a topology change one of them honours fails on the other; a clause \
         with a single channel is that engine's word for the document, not the document's own. \
         Add the sibling to {CPP_DRIVER}",
    );

    let cpp_only: Vec<&String> = cpp.difference(&rust).collect();
    assert!(
        cpp_only.is_empty(),
        "{cpp_only:?} are asserted only in {CPP_DRIVER}. Add the sibling to {RUST_DRIVER}",
    );
}

/// Both channels generate from the same file.
///
/// Name parity above is a claim about two suites asking the same questions;
/// it says nothing about what they are asking them OF. The C++ channel takes
/// its machine from a `sce_add_state_machine` registration and the Rust one
/// from `scripts/regen_ai_loop.sh`, so retargeting either leaves both suites
/// green while they describe different documents — and the failure would
/// surface later as a clause that "only one engine honours", which is exactly
/// the diagnosis this pairing exists to make trustworthy.
#[test]
fn both_channels_generate_from_the_same_document() {
    let regen = read("scripts/regen_ai_loop.sh");
    assert!(
        regen.contains(&format!("FIXTURE=\"{AI_LOOP_DOCUMENT}\"")),
        "scripts/regen_ai_loop.sh no longer generates the Rust channel from {AI_LOOP_DOCUMENT}",
    );

    let cmake = without_comments(&read("tests/CMakeLists.txt"));
    assert!(
        cmake.contains(&format!("${{CMAKE_SOURCE_DIR}}/{AI_LOOP_DOCUMENT}")),
        "tests/CMakeLists.txt no longer builds the C++ channel's machine from {AI_LOOP_DOCUMENT}",
    );
}
