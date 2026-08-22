// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! No single mutation casefile grows past what its CI job can finish.
//!
//! `mutation-rounds.yml` runs one job per CASEFILE, and the change that made
//! it a matrix rather than a loop is what brought the lane back inside its
//! interval — its header records the arithmetic: "wall clock becomes the
//! slowest single casefile instead of their sum, 39m rather than 1h37m". The
//! part that change cannot help with is a casefile that is large by itself.
//! `mutation_rounds_selection.cases` holds 21 cases and measured 4h16m against
//! a 330-minute job timeout, and every case added to it is spent inside one
//! job that nothing can split.
//!
//! ⚠ Case count is a PROXY for time, and a weak one: a `mutation_ctest` case
//! pays for a CMake configure and a full C++ build where a cargo case pays for
//! a crate. `error_cascade_is_bounded_ctest` holds 16 of the expensive kind
//! and `mutation_rounds_selection` 21 of the cheap kind, and it is the second
//! that is closest to the ceiling. The proxy is used anyway because it is the
//! only signal available WITHOUT running the corpus, and a check that needs
//! four hours to answer is a check nobody runs. What it buys is narrow and
//! worth stating: the largest casefile cannot grow silently.
//!
//! The remedy when this fires is a split, not a bigger number. The file that
//! trips it will be one that has accumulated several concerns —
//! `mutation_rounds_selection` covers the gate's selection, the lane's matrix,
//! the tree preparation and the service provisioning across five
//! `mutation_runtime_targets` — and splitting by concern hands the halves to
//! separate matrix jobs, which is the same remedy the lane already applied
//! once at the corpus level.

use std::path::{Path, PathBuf};

/// The largest casefile at the time this ceiling was written, and therefore
/// the ceiling: a ratchet, not a budget with room in it. Raising it is the
/// wrong repair — see the header.
const MAX_CASES_PER_FILE: usize = 21;

/// Floors, so a walk that stops finding things cannot pass by reading
/// nothing. Both were counted, not guessed: 73 casefiles holding 502 cases.
const CASEFILE_FLOOR: usize = 60;
const CASE_FLOOR: usize = 400;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

#[test]
fn no_casefile_outgrows_the_job_that_has_to_finish_it() {
    let dir = repo_root().join("sce-build/tests/mutations");
    let mut files = 0usize;
    let mut cases_total = 0usize;
    let mut oversized: Vec<(String, usize)> = Vec::new();

    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().is_none_or(|x| x != "cases") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        // The declaration, at the start of a line — the vocabulary the runner
        // itself parses. Casefile prose names it while explaining the format,
        // and a comment is not a case.
        let cases = text
            .lines()
            .filter(|l| l.starts_with("mutation_case "))
            .count();
        files += 1;
        cases_total += cases;
        if cases > MAX_CASES_PER_FILE {
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            oversized.push((name, cases));
        }
    }

    assert!(
        files >= CASEFILE_FLOOR && cases_total >= CASE_FLOOR,
        "read {files} casefile(s) holding {cases_total} case(s), expected at least \
         {CASEFILE_FLOOR} and {CASE_FLOOR}. The walk or the `mutation_case ` marker \
         moved — a corpus that reads as empty satisfies every ceiling"
    );

    oversized.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    assert!(
        oversized.is_empty(),
        "{oversized:?} hold more cases than one CI job is measured to finish \
         (ceiling {MAX_CASES_PER_FILE}). One casefile is one matrix job, so cases added \
         here are spent inside a job nothing can split, and the largest already \
         measured 4h16m against a 330-minute timeout. Split the file by concern \
         — its `mutation_runtime_targets` are usually the seam — rather than \
         raising the ceiling, which only moves the cancellation later."
    );
}
