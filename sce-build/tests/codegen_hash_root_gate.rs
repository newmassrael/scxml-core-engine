// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Build-graph gate: a codegen invocation whose SCXML input is STAGED
// into its own output directory must declare `--input-root`.
//
// §synth-6.2.6 defaults the source-hash root to the input file's parent
// directory. That default is right when the input sits in a tracked
// source tree (`resources/<n>/`, `tests/mesh/`), and wrong when the
// build first copies the input into the shared output directory,
// because the parent is then the output directory itself. Two failures
// follow, and the second one breaks builds:
//
//   1. The digest covers every fixture that happens to share the output
//      directory rather than this document's inputs, so unrelated tests
//      perturb each other's hashes.
//   2. `SourceSet::collect` reads that directory while sibling targets
//      are still staging into it. `read_dir` lists a file, a concurrent
//      target replaces it, `fs::read` returns ENOENT, and codegen exits
//      non-zero — an intermittent hard build failure whose frequency
//      tracks `-j`.
//
// Both harnesses below stage into a shared output directory, so every
// `generate` they issue names a tracked, build-quiescent root instead.
// This gate pins that: adding a seventh call site without the flag
// reintroduces the race, and the failure it causes is intermittent, so
// a reviewer cannot rely on seeing it.

use std::fs;
use std::path::{Path, PathBuf};

/// CMake harnesses that stage their SCXML inputs into the codegen output
/// directory. Every `sce-codegen generate` issued from these files needs
/// an explicit `--input-root`.
const STAGED_INPUT_HARNESSES: &[&str] = &[
    "cmake/SCEStaticW3CTest.cmake",
    "cmake/SCEStaticIntegrationFixture.cmake",
];

/// Marks the start of a codegen command in CMake.
const GENERATE_MARKER: &str = "generate";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Splits `content` into one window per `${SCE_CODEGEN}` ... `generate`
/// invocation. A window runs to the `DEPENDS` keyword that every
/// `add_custom_command` carries, which bounds the argument list whether
/// it was written on one line or wrapped across several.
///
/// The subcommand must follow `${SCE_CODEGEN}` immediately — only the
/// closing quote and whitespace may intervene. `${SCE_CODEGEN}` also
/// appears as a plain `DEPENDS` prerequisite, and scanning forward for
/// the next `generate` from there would land on prose or on an unrelated
/// `sce_generate_*` CMake function name and report it as an unflagged
/// call site.
///
/// `generate-conformance` / `generate-w3c` / `generate-integration` are
/// different subcommands with their own root handling and are skipped:
/// the token must end after `generate`.
fn generate_windows(content: &str) -> Vec<String> {
    let mut windows = Vec::new();
    for chunk in content.split("${SCE_CODEGEN}").skip(1) {
        // Step over the closing quote of the variable reference and any
        // wrapping whitespace, then demand the subcommand right there.
        let head = chunk.trim_start_matches('"').trim_start();
        let Some(args) = head.strip_prefix(GENERATE_MARKER) else {
            continue;
        };
        // Reject `generate-w3c` and friends: the char right after the
        // marker must not continue the token.
        if args
            .chars()
            .next()
            .is_some_and(|c| c == '-' || c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let window = match args.split_once("DEPENDS") {
            Some((a, _)) => a,
            None => args,
        };
        windows.push(window.to_string());
    }
    windows
}

#[test]
fn staged_input_codegen_declares_its_hash_root() {
    let root = repo_root();
    let mut offenders: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for rel in STAGED_INPUT_HARNESSES {
        let path = root.join(rel);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{rel}: harness must be readable: {e}"));

        for (idx, window) in generate_windows(&content).into_iter().enumerate() {
            checked += 1;
            if !window.contains("--input-root") {
                let first_line = window.lines().next().unwrap_or("").trim().to_string();
                offenders.push(format!("  {rel} (generate #{}): {first_line}", idx + 1));
            }
        }
    }

    assert!(
        checked > 0,
        "gate found no `${{SCE_CODEGEN}}` generate calls in {STAGED_INPUT_HARNESSES:?} — \
         the harnesses moved or the marker changed, so this gate is checking nothing"
    );

    assert!(
        offenders.is_empty(),
        "codegen invocations stage their input into the output directory but do not \
         declare `--input-root`, so §synth-6.2.6 hashes the output directory:\n{}\n\n\
         Pass `--input-root` naming the tracked source directory the input was staged \
         from (RESOURCE_DIR / FIXTURE_ROOT). Without it the source-hash covers unrelated \
         fixtures and the walk races concurrent staging (read_dir lists a file that \
         fs::read no longer finds), which fails the build intermittently under -j.",
        offenders.join("\n")
    );
}

/// The gate above is only meaningful if its window parser actually sees
/// the argument list. A parser that returned empty windows would pass
/// every file silently, so pin the shape it is expected to extract.
#[test]
fn generate_window_parser_sees_the_argument_list() {
    let sample = r#"
        add_custom_command(
            OUTPUT "${H}"
            COMMAND "${SCE_CODEGEN}" generate "${STAGED}" -l c11 -o "${OUT}" --input-root "${SRC}"
            DEPENDS "${STAGED}"
        )
        add_custom_command(
            OUTPUT "${H2}"
            COMMAND "${SCE_CODEGEN}" generate "${STAGED2}"
                    -l cpp -o "${OUT}"
            DEPENDS "${STAGED2}" "${SCE_CODEGEN}"
            COMMENT "sce_generate_static_thing: generate the thing"
        )
    "#;

    let windows = generate_windows(sample);
    assert_eq!(
        windows.len(),
        2,
        "one window per generate call — the `DEPENDS \"${{SCE_CODEGEN}}\"` prerequisite and \
         the `sce_generate_*` name in the comment must not register as call sites"
    );
    assert!(
        windows[0].contains("--input-root"),
        "single-line form parsed"
    );
    assert!(
        !windows[1].contains("--input-root"),
        "wrapped form parsed and its missing flag detected"
    );
    assert!(
        windows[1].contains("-l cpp"),
        "window must span the wrapped continuation lines, not stop at the newline"
    );
}

/// Subcommands that are not bare `generate` carry their own root rules
/// and must not be dragged into the gate.
#[test]
fn sibling_subcommands_are_not_gated() {
    let sample = r#"
            COMMAND "${SCE_CODEGEN}" generate-conformance -o "${OUT}"
            DEPENDS x
            COMMAND "${SCE_CODEGEN}" generate-w3c -l go
            DEPENDS y
    "#;
    assert!(
        generate_windows(sample).is_empty(),
        "`generate-conformance` / `generate-w3c` must not match the bare `generate` marker"
    );
}
