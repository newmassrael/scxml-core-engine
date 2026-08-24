// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The AI supervision loop is asked the same questions by every engine that
//! drives it.
//!
//! `examples/ai_loop/ai_loop.scxml` is deliberately outside the
//! seven-channel stem contract in `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md`:
//! it is a worked example, and `integration_stem_registration` enumerates
//! `integration_resources/` and so does not reach it. Nothing else would
//! notice a channel drifting from the document either.
//!
//! Measured 2026-08-22: the Rust channel asserted 19 clauses and the C++ AOT
//! channel 15 — four clauses were the word of one engine only, among them the
//! one this document exists to demonstrate
//! (`§scxml-D-addAncestorStatesToEnter`, whose defect was found on the AOT
//! engines with every W3C fixture green). A clause asserted in one channel is
//! not a claim about the document, and the drift is invisible because every
//! suite stays green while it widens.
//!
//! So this pins the pairing rather than the count: every scenario in any
//! registered channel has a counterpart in all the others. Adding one to a
//! single side fails here, which is the moment it is cheapest to fix.
//!
//! `CHANNELS` is the registry, and it is deliberately a list rather than the
//! two named constants this file used to hold. The claim being made grew from
//! "two engines, one document" to "every engine that runs this document is
//! asked the same things", and a pair of constants cannot express the second:
//! adding a third channel meant editing every assertion, which is the shape
//! that keeps a suite at two. Registering a channel is now one entry, and the
//! entry carries everything the checks need — where the driver is, how its
//! scenario names are spelled, and what points its machine at the document.
//!
//! Each driver spells its scenarios in its own language's convention
//! (`answering_a_question_does_not_re_prime_the_session`,
//! `AnsweringAQuestionDoesNotRePrimeTheSession` and
//! `TestAnsweringAQuestionDoesNotRePrimeTheSession`), so the pairing is by
//! normalised name. That makes the identifier itself the contract: the drivers
//! already name each scenario after the clause it asserts, and matching text
//! is what lets a reader move between the channels.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The document every channel drives. Named here rather than derived, so a
/// regen script or a CMake registration that quietly retargets one channel
/// fails against a written-down answer instead of agreeing with itself.
const AI_LOOP_DOCUMENT: &str = "examples/ai_loop/ai_loop.scxml";

/// One channel: an engine driving `AI_LOOP_DOCUMENT`, and everything the
/// checks below need in order to hold it to the same scenario set as its
/// siblings.
struct Channel {
    /// How a failure names this channel to a reader.
    engine: &'static str,
    /// The hand-authored driver, repo-relative.
    driver: &'static str,
    /// A regex whose first capture group is a scenario's name as that
    /// language spells it. Per channel because the frameworks differ, and the
    /// Go one has to drop the `Test` prefix the toolchain requires so the name
    /// left over is the clause.
    scenario: &'static str,
    /// The file that points this channel's machine at the document.
    generator: &'static str,
    /// The text `generator` must contain. Spelled per channel because a CMake
    /// registration and a shell script name the same file differently, and
    /// checked against `AI_LOOP_DOCUMENT` below — so a channel cannot be
    /// registered with a reference that names some other document.
    names_document: &'static str,
}

/// Every engine that drives the worked example.
///
/// The Go entry landed 2026-08-25 and earned itself on its first run: with the
/// Rust and C++ channels green on all 27 clauses, it failed
/// `a_verdict_without_its_payload_is_reported`, because the Go Lua engine bound
/// `_event.data` as the empty STRING for a payload-less event where every other
/// engine leaves it nil. Lua gives strings a metatable, so the document's
/// `cond` read a field off it and answered false instead of raising
/// `error.execution` — a supervising host could not tell "the verdict said no"
/// from "the verdict never carried one". That is what a third channel is for,
/// and it is why the registry exists rather than a pair of constants.
///
/// The Kotlin entry landed the same day and did it again, twice over. It was
/// the first channel to resolve an external transition's domain the way
/// §scxml-D-getTransitionDomain does — `findLCCA` filters ancestors to `<state>`
/// and `<scxml>`, so a `<parallel>` is never a domain — and under that rule the
/// document's own `<transition event="session.lost">`, written on a region root
/// and left at the default `external` type, exited every region and preempted
/// the liveness watch's answer to the same event. The document was ambiguous
/// and the four engines had been reading it two ways; it now says
/// `type="internal"`, which is what its comment always claimed. Then, with the
/// document fixed, the Kotlin engine's own `InternalToTarget` branch turned out
/// not to snapshot the configuration before exiting, so `<history>` recorded
/// the run's position from one transition earlier.
///
/// The Python entry landed the same day and is the first that needed no repair:
/// 27 of 27 on its first run. That is worth recording rather than passing over,
/// because it is what the other four make measurable — this engine already
/// leaves `_event.data` unbound for a payload-less event (the Go defect), keeps
/// the terminal in the configuration (where Kotlin follows `exitInterpreter` and
/// empties it), and takes both regions' transitions on the corrected document.
const CHANNELS: &[Channel] = &[
    Channel {
        engine: "Rust AOT",
        driver: "backends/rust/tests/tests/ai_loop.rs",
        scenario: r"#\[test\]\s+fn\s+(\w+)",
        generator: "scripts/regen_ai_loop.sh",
        names_document: "FIXTURE=\"examples/ai_loop/ai_loop.scxml\"",
    },
    Channel {
        engine: "C++ AOT",
        driver: "tests/integration/AiLoopAotTest.cpp",
        scenario: r"TEST_F\(\s*AiLoopAotTest\s*,\s*(\w+)\s*\)",
        generator: "tests/CMakeLists.txt",
        names_document: "${CMAKE_SOURCE_DIR}/examples/ai_loop/ai_loop.scxml",
    },
    Channel {
        engine: "Go AOT",
        driver: "backends/go/tests/integration/ai_loop/ai_loop_test.go",
        scenario: r"func\s+Test(\w+)\(t \*testing\.T\)",
        generator: "scripts/regen_ai_loop_go.sh",
        names_document: "FIXTURE=\"examples/ai_loop/ai_loop.scxml\"",
    },
    Channel {
        engine: "Kotlin AOT",
        driver: "backends/kotlin/tests/src/test/kotlin/com/sce/integration/AiLoopTest.kt",
        scenario: r"@Test\s+fun\s+(\w+)\s*\(",
        generator: "scripts/regen_ai_loop_kotlin.sh",
        names_document: "FIXTURE=\"examples/ai_loop/ai_loop.scxml\"",
    },
    Channel {
        engine: "Python AOT",
        driver: "backends/python/tests/integration/ai_loop/test_ai_loop_aot.py",
        scenario: r"def\s+test_(\w+)\s*\(",
        generator: "scripts/regen_ai_loop_python.sh",
        names_document: "FIXTURE=\"examples/ai_loop/ai_loop.scxml\"",
    },
];

/// What every channel asserted when the pairing was last raised. A scanner
/// that stops matching reports empty sets, and empty sets pair perfectly — the
/// floor is what makes a broken pattern fail instead of pass. It is a ratchet:
/// raise it when the suite grows, never lower it to accommodate a deletion.
///
/// 19 when the pairing landed (2026-08-22); 24 since 2026-08-23, when counting
/// which of the document's five enumerated outcomes any channel actually
/// reached found `converged` and `failed` at none, `stuck` — one of the two
/// ways into `exhausted` — at none, and both channels driving `judge` without
/// the payload its `cond` reads; 25 when the document's sends became acts a
/// host serves (§scxml-6.2.5) and needed a scenario that RECORDS them, since
/// the silent handler every other scenario registers cannot tell a declared act
/// from a targetless send that lost its `type`; 27 since 2026-08-24, when the
/// resume seam (§scxml-3.2 `enterAt` / `get_state_from_name`) reached the C++
/// engine and the two scenarios that had been the Rust channel's word alone got
/// their siblings.
const SCENARIO_FLOOR: usize = 27;

/// A registry holding one channel pairs with itself, and a registry holding
/// none pairs vacuously — both would report agreement about a document nobody
/// drives. Five is what the claim is worth today, and like `SCENARIO_FLOOR`
/// it is a ratchet: raise it when a channel lands, never lower it to
/// accommodate one being dropped.
const CHANNEL_FLOOR: usize = 5;

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
/// The drivers name each other's scenarios in prose — that cross reference is
/// the point of the header comments — so a scanner reading comments would pair
/// the channels against their own documentation and report agreement no matter
/// what the code did. The generators are worse: every one of them explains in
/// its header which document it regenerates, so a scan that kept comments would
/// accept a script whose only mention of the document is the sentence saying it
/// used to build it.
///
/// `#` opens a comment in both shell and CMake, which is why it is here at all.
/// `#[` is exempt because it opens a Rust ATTRIBUTE, and `#[test]` is the first
/// half of what the Rust channel's own pattern matches — stripping it would
/// leave that channel reporting no scenarios, which is the failure this
/// function's floor exists to catch rather than to cause.
fn without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            let shell_or_cmake = trimmed.starts_with('#') && !trimmed.starts_with("#[");
            !(trimmed.starts_with("//") || shell_or_cmake)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `answering_a_question…`, `AnsweringAQuestion…` and `TestAnsweringAQuestion…`
/// are the same clause spelled in three languages' conventions.
fn normalise(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

impl Channel {
    fn scenarios(&self) -> BTreeSet<String> {
        let source = read(self.driver);
        let re = regex::Regex::new(self.scenario).expect("scenario pattern");
        re.captures_iter(&without_comments(&source))
            .map(|c| normalise(&c[1]))
            .collect()
    }
}

/// Every registered channel's scenario set, keyed by the engine's own name so
/// a failure can say which one is missing a clause.
fn by_engine() -> BTreeMap<&'static str, BTreeSet<String>> {
    CHANNELS
        .iter()
        .map(|channel| (channel.engine, channel.scenarios()))
        .collect()
}

#[test]
fn every_ai_loop_clause_is_asserted_by_every_engine() {
    assert!(
        CHANNELS.len() >= CHANNEL_FLOOR,
        "{} channel(s) are registered and the claim this file makes needs at least \
         {CHANNEL_FLOOR}; a registry that shrank pairs its remaining channels with \
         themselves and reports agreement about a document nobody else drives",
        CHANNELS.len(),
    );

    let found = by_engine();

    for (engine, scenarios) in &found {
        assert!(
            scenarios.len() >= SCENARIO_FLOOR,
            "the {engine} channel asserts {} scenario(s) and every channel must assert at \
             least {SCENARIO_FLOOR}. Either its driver lost scenarios, or the pattern \
             reading it stopped matching — and sets that pair because they are empty prove \
             nothing",
            scenarios.len(),
        );
    }

    // The union rather than a pairwise sweep: a clause missing from two
    // channels should be reported against both, and comparing each channel to
    // what the document as a whole claims is what does that.
    let every_clause: BTreeSet<String> = found.values().flatten().cloned().collect();

    for channel in CHANNELS {
        let scenarios = &found[channel.engine];
        let missing: Vec<&String> = every_clause.difference(scenarios).collect();
        assert!(
            missing.is_empty(),
            "{missing:?} are asserted by another channel and not by {}. \
             {AI_LOOP_DOCUMENT} is driven by {} engines precisely so a topology change one \
             of them honours fails on the others; a clause with a single channel is that \
             engine's word for the document, not the document's own. Add the sibling to {}",
            channel.engine,
            CHANNELS.len(),
            channel.driver,
        );
    }
}

/// Every channel generates from the same file.
///
/// Name parity above is a claim about several suites asking the same
/// questions; it says nothing about what they are asking them OF. The C++
/// channel takes its machine from a `sce_add_state_machine` registration and
/// the Rust and Go ones from their regen scripts, so retargeting any of them
/// leaves every suite green while they describe different documents — and the
/// failure would surface later as a clause that "only one engine honours",
/// which is exactly the diagnosis this pairing exists to make trustworthy.
#[test]
fn every_channel_generates_from_the_same_document() {
    assert!(
        CHANNELS.len() >= CHANNEL_FLOOR,
        "{} channel(s) are registered and at least {CHANNEL_FLOOR} are expected; a sweep \
         over an emptied registry checks nothing and passes",
        CHANNELS.len(),
    );

    for channel in CHANNELS {
        assert!(
            channel.names_document.contains(AI_LOOP_DOCUMENT),
            "the {} channel is registered as generating from `{}`, which does not name \
             {AI_LOOP_DOCUMENT}. The per-channel spelling exists because a CMake \
             registration and a shell script write the same path differently, not so a \
             channel can point somewhere else",
            channel.engine,
            channel.names_document,
        );

        let generator = without_comments(&read(channel.generator));
        assert!(
            generator.contains(channel.names_document),
            "{} no longer builds the {} channel's machine from {AI_LOOP_DOCUMENT} — \
             `{}` is not in it",
            channel.generator,
            channel.engine,
            channel.names_document,
        );
    }
}
