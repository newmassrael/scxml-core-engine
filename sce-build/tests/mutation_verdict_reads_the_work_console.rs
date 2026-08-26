// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The verdict reads the working console; the cap touches only the kept copy.
//
// `mutation_kept_console_is_bounded.rs` measures this by running a round and
// checking that a name buried mid-flood survives into the record. That is the
// behavioural half and it is the one that matters — but it needs a nine-megabyte
// fixture and a build, so it is the half a hurried change is tempted to skip.
// This file pins the same fact structurally, out of the script's own text, in
// milliseconds: `mutation_failures_from_gtest` is fed from
// `$WORK/ctest-console.txt`, and `mutation_copy_capped` is called on exactly one
// destination, the copy under `SCE_MUTATION_REPORT_DIR`.
//
// Two rules this repository paid for shape how the scanning is done.
//
// COMMENTS ARE STRIPPED FIRST. A scanner satisfied by the prose around the code
// has happened here twice in one hour, and this file is a prime candidate: the
// paragraph above names both paths, and `scripts/mutate` explains the
// distinction at length in its own comments. Only lines that are not comments
// are searched.
//
// AND THERE IS A FLOOR. A source scan whose population goes empty reports zero
// violations, which reads exactly like a clean tree — so the line count is
// asserted before anything else. If `scripts/mutate` is ever rewritten in
// another language, or moved, this fails loudly instead of passing vacuously.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// `scripts/mutate` with comment-only lines removed, as (1-based line number,
/// text). The numbers are kept so a failure can say where.
fn code_lines() -> Vec<(usize, String)> {
    let path = repo_root().join("scripts/mutate");
    let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    body.lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.to_string()))
        .filter(|(_, line)| !line.trim_start().starts_with('#'))
        .filter(|(_, line)| !line.trim().is_empty())
        .collect()
}

fn matching<'a>(code: &'a [(usize, String)], needle: &str) -> Vec<&'a (usize, String)> {
    code.iter().filter(|(_, l)| l.contains(needle)).collect()
}

#[test]
fn the_gtest_parser_is_fed_the_working_console_and_nothing_else() {
    let code = code_lines();
    // The floor, first. Everything below counts occurrences, and a count of
    // zero out of zero lines is the shape that reads as a pass.
    assert!(
        code.len() > 500,
        "only {} non-comment lines were read out of scripts/mutate — this scan \
         has no population and would pass on anything",
        code.len()
    );

    let fed = matching(&code, "mutation_failures_from_gtest");
    assert_eq!(
        fed.len(),
        1,
        "expected exactly one place to read gtest's verdict lines, found: {fed:?}"
    );
    let (line, text) = fed[0];
    assert!(
        text.contains("< \"$console\""),
        "scripts/mutate:{line} feeds the gtest parser from something other than \
         $console:\n  {text}"
    );

    // And `$console`, in the function that does the feeding, is the working
    // copy. Asserted on the assignment rather than on a comment, because the
    // comment is what a wrong change leaves untouched.
    let assigned = matching(&code, "console=\"$WORK/ctest-console.txt\"");
    assert_eq!(
        assigned.len(),
        1,
        "expected exactly one assignment of the working ctest console, found: {assigned:?}"
    );
}

#[test]
fn the_cap_is_applied_to_the_kept_copy_and_only_to_it() {
    let code = code_lines();
    assert!(
        code.len() > 500,
        "only {} non-comment lines were read — this scan has no population",
        code.len()
    );

    let uses = matching(&code, "mutation_copy_capped");
    // Its definition and one call. A second call site is not automatically
    // wrong, but it is not something to discover from a disk that filled or a
    // `red:` list that quietly shortened.
    assert_eq!(
        uses.len(),
        2,
        "expected mutation_copy_capped to be defined once and called once, found: {uses:?}"
    );
    let definition = uses
        .iter()
        .find(|(_, l)| l.starts_with("mutation_copy_capped()"))
        .unwrap_or_else(|| panic!("mutation_copy_capped has no definition among {uses:?}"));
    let call = uses
        .iter()
        .find(|(n, _)| n != &definition.0)
        .expect("one call site");

    assert!(
        call.1.contains("\"$kept.console.txt\""),
        "scripts/mutate:{} caps something other than the kept console:\n  {}",
        call.0,
        call.1
    );
    // The mistake this whole pair exists to catch, named so a reader of a
    // failure knows what went wrong rather than only that something did.
    assert!(
        !call.1.contains("\"$console\""),
        "scripts/mutate:{} applies the cap to the console a VERDICT is read \
         from. The `red:` list would silently lose any test that failed \
         mid-flood — cap the copy under SCE_MUTATION_REPORT_DIR instead:\n  {}",
        call.0,
        call.1
    );
}

#[test]
fn the_kept_console_is_written_and_never_read() {
    let code = code_lines();
    assert!(code.len() > 500, "no population: {} lines", code.len());

    // `$kept.console.txt` is the kept copy's path. It appears once, as the
    // destination of the capped copy — which is what "nothing reads it" looks
    // like when asserted rather than asserted-about.
    let kept = matching(&code, "$kept.console.txt");
    assert_eq!(
        kept.len(),
        1,
        "the kept console path is used more than once; if something now READS \
         it, the cap above is no longer safe: {kept:?}"
    );
    assert!(
        kept[0].1.trim().starts_with("mutation_copy_capped"),
        "scripts/mutate:{} mentions the kept console somewhere other than the \
         copy that writes it:\n  {}",
        kept[0].0,
        kept[0].1
    );

    // The three artefacts that told G7's `ran 0 tests` apart from a timeout are
    // copied whole, and this says so: a future cap reaching any of them would
    // be capping the evidence rather than the flood.
    for uncapped in [
        "$kept.junit.xml",
        "$kept.junit.missing",
        "$kept.runner-stopped.txt",
    ] {
        for (line, text) in matching(&code, uncapped) {
            assert!(
                !text.contains("mutation_copy_capped"),
                "scripts/mutate:{line} caps {uncapped}, which is one of the three \
                 artefacts that made G7 diagnosable:\n  {text}"
            );
        }
    }
}
