// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every CMake step that runs the code generator must declare the
// templates as an input.
//
// A generated source file depends on two things: the SCXML it was
// rendered from, and the ~120 jinja2 templates that rendered it. CMake
// is told about the first by `DEPENDS` and about the second only by a
// `DEPFILE`, which `sce-codegen --write-deps` writes. A step missing the
// depfile does not fail loudly — it silently reuses a stale artefact
// after a template edit, so a template fix appears not to work and the
// build reports success. Measured before this gate existed:
//
//   editing `templates/state_machine.jinja2`      →  0 of 21 C++
//     integration artefacts regenerated
//   editing `templates/c/state_machine.c.jinja2`  → 74 of 270 C11
//     artefacts regenerated (the rest reached only via a dependency
//     chain, which is coincidence rather than declared dependency)
//
// The defect cost real time: a C11 template fix in this repo had to be
// forced through with a manual `rm` of the generated `.c`, twice, before
// the cause was found. `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` claimed
// "the build itself is the §6.2.6 freshness invariant" for the C11
// backend, which was false for exactly this reason.
//
// This is a source-scanning gate, so it carries a floor: a scan that
// silently matches nothing reads every bit as green as a correct tree.

use std::path::{Path, PathBuf};

/// CMake files defining codegen steps. Listed rather than globbed so a
/// new generator file is a deliberate addition here, not something that
/// slips in unscanned.
const CMAKE_FILES: &[&str] = &[
    "cmake/SCEStaticW3CTest.cmake",
    "cmake/SCEStaticIntegrationFixture.cmake",
    "cmake/SCECodegen.cmake",
];

/// Measured lower bound on codegen invocations across those files —
/// read off a run with the floor set impossibly high, not guessed. A
/// generator that stops being scanned (renamed file, changed invocation
/// spelling) drops below it.
///
/// Coverage was then measured per site rather than assumed: deleting any
/// one of the 10 `DEPFILE` keywords individually turns this gate red.
const MIN_CODEGEN_INVOCATIONS: usize = 12;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Split a CMake file into `add_custom_command(...)` bodies by tracking
/// parenthesis depth. A brace-counting split rather than a regex because
/// these blocks nest quoted parens (generator expressions, shell
/// fragments) that a non-greedy match would cut short — the same failure
/// mode that made an earlier literal-scanning gate report truncated
/// values as correct.
fn custom_command_bodies(text: &str) -> Vec<String> {
    const OPEN: &str = "add_custom_command(";
    let mut out = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut search_from = 0usize;

    while let Some(rel) = text[search_from..].find(OPEN) {
        let start_byte = search_from + rel + OPEN.len();
        let start = text[..start_byte].chars().count();
        let mut depth = 1usize;
        let mut i = start;
        let mut in_quote = false;
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                '"' => in_quote = !in_quote,
                '(' if !in_quote => depth += 1,
                ')' if !in_quote => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        out.push(bytes[start..i.saturating_sub(1)].iter().collect());
        search_from = start_byte;
    }
    out
}

/// Does this command body invoke the generator?
///
/// Two spellings, and matching only the first would have left a whole
/// file unscanned: `SCEStaticW3CTest.cmake` inlines the binary and its
/// subcommand on the COMMAND line, while `SCECodegen.cmake` assembles
/// them into `_SCE_CODEGEN_CMD` beforehand and expands the variable.
/// Targeting one spelling reads as full coverage while silently skipping
/// the other — the failure mode of aiming a scan at a single name.
fn runs_codegen(body: &str) -> bool {
    body.contains("SCE_CODEGEN}\" generate")
        || body.contains("SCE_CODEGEN} generate")
        || body.contains("_SCE_CODEGEN_CMD}")
}

#[test]
fn every_codegen_step_declares_its_template_inputs() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    for rel in CMAKE_FILES {
        let path: &Path = &root.join(rel);
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("{rel} is readable: {e} — the gate cannot scan what it cannot open")
        });

        for body in custom_command_bodies(&text) {
            if !runs_codegen(&body) {
                continue;
            }
            scanned += 1;

            let output = body
                .lines()
                .find_map(|l| l.trim().strip_prefix("OUTPUT "))
                .unwrap_or("<unnamed>")
                .trim()
                .to_string();

            // Both halves are required and neither implies the other:
            // `--write-deps` without `DEPFILE` writes a file nobody
            // reads, and `DEPFILE` without `--write-deps` points CMake
            // at a file that is never produced.
            //
            // `--write-deps` is looked up at file scope, not in the
            // command body, because a generator may build its argv into
            // a variable first (`SCECodegen.cmake`) — checking the body
            // alone reported that file as violating when it is in fact
            // the pattern the others were brought up to.
            let writes = body.contains("--write-deps") || text.contains("--write-deps");
            // `DEPFILE` as a CMake keyword, which is an argument of its
            // own on its own line — not as a substring. Substring
            // matching passed unconditionally because the depfile paths
            // are held in variables *named* `_PARENT_DEPFILE` /
            // `_CHILD_DEPFILE`, so `--write-deps "${_PARENT_DEPFILE}"`
            // satisfied the check on its own. Deleting the real keyword
            // left the gate green; only mutating each fixed site
            // individually surfaced it.
            let declares = body.lines().any(|l| l.trim_start().starts_with("DEPFILE "));
            if !writes || !declares {
                violations.push(format!(
                    "{rel}: codegen step for {output} is missing {}\n  \
                     A template edit will leave its output stale and the build will \
                     reuse it silently. Add `--write-deps \"<output>.d\"` to the \
                     COMMAND and `DEPFILE \"<output>.d\"` to the command, as \
                     SCECodegen.cmake does.",
                    match (writes, declares) {
                        (false, false) => "both `--write-deps` and `DEPFILE`",
                        (false, true) => "`--write-deps` (DEPFILE names a file nothing writes)",
                        _ => "`DEPFILE` (the depfile is written but never read)",
                    }
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "codegen steps do not declare their template inputs:\n\n{}",
        violations.join("\n\n"),
    );

    assert!(
        scanned >= MIN_CODEGEN_INVOCATIONS,
        "scanned only {scanned} codegen invocations, floor {MIN_CODEGEN_INVOCATIONS} — \
         the scan stopped matching, so a green result here means nothing was checked \
         rather than that everything passed",
    );
}
