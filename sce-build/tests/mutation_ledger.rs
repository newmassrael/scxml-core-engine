// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A mutation round's verdict survives the session that ran it.
//
// `scripts/lib/mutation_ledger.sh` writes each round's result to a path
// outside the tree, and `scripts/mutation-ledger` reads it back to answer the
// question a session starts with: which casefiles has nobody run yet. Its
// header carries why that has to be a mechanism rather than a habit — twice, a
// verdict existed only in the scratch directory of a session that then ended.
//
// Every failure this file guards against reads as success:
//
//   - a ledger path that moves with whoever is running puts the record
//     somewhere the next session does not look, and the round still prints
//     `3/3 caught`. That is not hypothetical: the first shape of the path rule
//     used `$XDG_DATA_HOME`, and the loop harness that drives these rounds
//     exports its own, so the very first round written through the library
//     landed under a per-run directory. The round looked perfect;
//   - `--check` recording a verdict would mark all 86 casefiles judged on the
//     next push, without a single test having run, and the corpus would read
//     as finished;
//   - an assertion counted as a measurement would retire a casefile nobody
//     measured, which is the exact laundering the `provenance` field exists to
//     prevent.
//
// The library is exercised by sourcing it, the way `scripts/mutate` does, so
// what is measured here is the code that runs in a round and not a copy of it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

mod common;
// Shared with `ledger_holes_are_still_declared`, which is a separate suite for
// a reason that is not subject matter — see the note on its own header. Both
// need to seed a round and then ask the tool about it, and a second copy of
// either half would be a second answer to what the ledger holds.
use common::ledger::{ask, labels_of, repo_root, with_library};

/// The ledger's location, as the library computes it under a given `HOME`.
fn ledger_dir(home: &Path) -> String {
    with_library(home, "mutation_ledger_dir").trim().to_string()
}

#[test]
fn the_ledger_does_not_move_with_the_harness_that_runs_the_round() {
    let home = tempdir().expect("temp home");

    // `with_library` sets `XDG_DATA_HOME` to a directory that is not the
    // answer. A path rule that consults it would return that one, which is
    // what the first version of this library did — and the loop harness that
    // drives these rounds sets exactly that variable, so the record went to a
    // directory named after the run rather than after the corpus.
    let dir = ledger_dir(home.path());

    assert_eq!(
        dir,
        home.path()
            .join(".local/share/sce-mutation-corpus/verdicts")
            .display()
            .to_string(),
        "the ledger path is derived from something other than HOME, so it \
         moves with whoever runs the round — which is a session id by \
         another name"
    );
    assert!(
        !dir.contains("decoy"),
        "the ledger followed XDG_DATA_HOME: {dir}"
    );
}

#[test]
fn an_explicit_ledger_directory_wins() {
    let home = tempdir().expect("temp home");
    let elsewhere = tempdir().expect("temp elsewhere");
    let out = Command::new("bash")
        .arg("-c")
        .arg("set -euo pipefail; source scripts/lib/mutation_ledger.sh; mutation_ledger_dir")
        .current_dir(repo_root())
        .env("HOME", home.path())
        .env("SCE_MUTATION_LEDGER_DIR", elsewhere.path())
        .output()
        .expect("run bash with the ledger library sourced");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        elsewhere.path().display().to_string(),
        "the override a test or a one-off round needs is not honoured"
    );
}

/// One round, recorded, parsed back out of the ledger.
fn record_a_round(home: &Path, casefile: &str, rc: &str) -> serde_json::Value {
    with_library(
        home,
        &format!(
            "rows=\"$(mktemp)\"; \
             mutation_ledger_begin {casefile} \"$rows\"; \
             mutation_ledger_case CAUGHT 'a guard stops guarding' '1/9 red' /dev/null; \
             mutation_ledger_case SURVIVED 'a second guard does not' '0/9 red'; \
             mutation_ledger_commit cargo {rc} >/dev/null; \
             rm -f \"$rows\""
        ),
    );
    let stem = Path::new(casefile)
        .file_stem()
        .expect("the casefile has a stem")
        .to_string_lossy()
        .into_owned();
    let text = fs::read_to_string(Path::new(&ledger_dir(home)).join(format!("{stem}.jsonl")))
        .expect("the round left a record");
    // The last line, because the ledger appends: a helper that read the whole
    // file as one object would work exactly until a second round was recorded,
    // which is the case the test below is about.
    let line = text
        .lines()
        .next_back()
        .expect("the round left at least one record");
    serde_json::from_str(line).expect("the record is one JSON object per line")
}

#[test]
fn a_recorded_round_names_the_tree_the_status_and_every_case() {
    let home = tempdir().expect("temp home");
    let record = record_a_round(
        home.path(),
        "sce-build/tests/mutations/path_naming_parity.cases",
        "1",
    );

    // The three things a later reader cannot reconstruct and must not have to
    // guess: which tree the round measured, whether it ended clean, and what
    // each case actually said. A record missing any one of them sends the
    // next session back to run the round again, which is the cost this whole
    // mechanism exists to avoid.
    assert_eq!(record["stem"], "path_naming_parity");
    assert_eq!(record["rc"], 1);
    assert_eq!(record["provenance"], "live");
    assert_eq!(
        record["tree"].as_str().expect("a tree").len(),
        40,
        "the record does not name the commit it measured"
    );
    assert_eq!(record["caught"], 1);
    assert_eq!(record["survived"], 1);
    assert_eq!(record["inconclusive"], 0);

    let cases = record["cases"].as_array().expect("per-case verdicts");
    assert_eq!(cases.len(), 2, "a case went unrecorded: {cases:?}");
    assert_eq!(cases[0]["verdict"], "CAUGHT");
    assert_eq!(cases[0]["label"], "a guard stops guarding");
    assert_eq!(cases[0]["detail"], "1/9 red");
    assert_eq!(cases[1]["verdict"], "SURVIVED");

    // A casefile that has been edited since must be visibly stale rather than
    // silently current, and the blob hash is the only thing that can say so.
    assert_eq!(
        record["casefile_blob"].as_str().expect("a blob").len(),
        40,
        "the record cannot tell whether the casefile has moved under it"
    );
}

#[test]
fn a_multi_line_detail_does_not_cost_the_round_its_record() {
    // An INCONCLUSIVE carries the compiler's own refusal, which is a
    // paragraph, and one row of this ledger is one LINE. The two met on
    // 2026-08-30: a six-case round with two refusals wrote rows whose first
    // line held two separators instead of three, the reader raised `not enough
    // values to unpack (expected 4, got 3)`, and ALL SIX verdicts were lost —
    // `recorded:` printed an empty path under a round that had just measured
    // something. The terminal still had the verdicts; the record, which is the
    // part a later session reads, had nothing.
    let home = tempdir().expect("temp home");
    let refusal = "mutated tree does not compile; the tests never ran\n\
                   error: unused variable: `assigned`\n   --> a/b.rs:132:9";
    with_library(
        home.path(),
        "rows=\"$(mktemp)\"; \
         mutation_ledger_begin sce-build/tests/mutations/a_paragraph.cases \"$rows\"; \
         mutation_ledger_case INCONCLUSIVE 'a case the compiler refused' \
             $'mutated tree does not compile; the tests never ran\\nerror: unused variable: `assigned`\\n   --> a/b.rs:132:9'; \
         mutation_ledger_case CAUGHT 'a case that landed' '1/8 red'; \
         mutation_ledger_commit cargo 1 >/dev/null; \
         rm -f \"$rows\"",
    );

    let text = fs::read_to_string(Path::new(&ledger_dir(home.path())).join("a_paragraph.jsonl"))
        .expect("the round left a record at all — this is the half that was lost");
    assert_eq!(
        text.lines().count(),
        1,
        "one round is one JSON line, whatever its details contain:\n{text}"
    );
    let record: serde_json::Value =
        serde_json::from_str(text.trim()).expect("the record is one JSON object per line");

    // Both cases, and the tally that goes with them: a reader that dropped the
    // malformed row alone would still under-report the round.
    assert_eq!(record["inconclusive"], 1);
    assert_eq!(record["caught"], 1);
    let cases = record["cases"].as_array().expect("per-case verdicts");
    assert_eq!(cases.len(), 2, "a case went unrecorded: {cases:?}");
    assert!(
        cases.iter().all(|c| c["verdict"] != "UNREADABLE"),
        "a row this reader could not parse survived into the record: {cases:?}"
    );

    // And the paragraph is a paragraph again. Folding it to a single line at
    // write time would keep the record — and lose the shape that makes a
    // compiler refusal readable, which is the reason it is quoted at all.
    assert_eq!(
        cases[0]["detail"].as_str().expect("the refusal"),
        refusal,
        "the refusal reached the record with its lines rearranged"
    );
}

#[test]
fn a_second_round_is_appended_rather_than_replacing_the_first() {
    let home = tempdir().expect("temp home");
    let casefile = "sce-build/tests/mutations/driver_fixture_citation.cases";
    record_a_round(home.path(), casefile, "0");
    record_a_round(home.path(), casefile, "1");

    let text = fs::read_to_string(
        Path::new(&ledger_dir(home.path())).join("driver_fixture_citation.jsonl"),
    )
    .expect("the rounds left records");

    // A case CAUGHT in June and SURVIVED in August is a regression, and a
    // ledger that kept only the newest verdict would erase the pair that says
    // so.
    assert_eq!(
        text.lines().count(),
        2,
        "the second round overwrote the first:\n{text}"
    );
}

struct Fixture {
    _dir: TempDir,
    casefile: PathBuf,
}

/// A casefile sound enough for `--check` to accept, in a directory of its own.
fn sound_fixture() -> Fixture {
    let dir = tempdir().expect("temp dir");
    let target = dir.path().join("subject.txt");
    fs::write(&target, "fn keep(x: u8) -> u8 {\n    x + 1\n}\n").expect("write the subject");
    let oracle = dir.path().join("oracle.txt");
    fs::write(&oracle, "the assertions that would catch it\n").expect("write the oracle");

    let path = target.display().to_string();
    let casefile = dir.path().join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "mutation_tests -p sce-build --features cli --lib\n\
             mutation_targets {path}\n\
             mutation_oracles {}\n\n\
             mutation_case \"the increment stops incrementing\" <<'PY'\n\
             edit({path:?}, \"x + 1\", \"x\")\n\
             PY\n",
            oracle.display()
        ),
    )
    .expect("write the casefile");

    Fixture {
        _dir: dir,
        casefile,
    }
}

#[test]
fn a_check_writes_no_verdict_because_it_read_none() {
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");
    let fixture = sound_fixture();

    let out = Command::new(repo_root().join("scripts/mutate"))
        .arg("--check")
        .arg(&fixture.casefile)
        .current_dir(repo_root())
        .env("HOME", home.path())
        .env("SCE_MUTATION_LEDGER_DIR", ledger.path())
        .output()
        .expect("run scripts/mutate --check");
    assert!(
        out.status.success(),
        "the fixture was supposed to be sound:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // `mutation-cases` runs this mode over every casefile on every push. If it
    // left records, one push would mark the whole corpus judged without a
    // single test having run — and the corpus would then read as finished.
    let left: Vec<_> = fs::read_dir(ledger.path())
        .expect("read the ledger directory")
        .map(|entry| entry.expect("a directory entry").file_name())
        .collect();
    assert!(
        left.is_empty(),
        "a mode that builds nothing and runs no test recorded a verdict: {left:?}"
    );
}

/// A casefile the corpus really holds, so the queries below are asked about
/// something `scripts/mutation-ledger` derives from the tree rather than from
/// a list this test wrote.
const A_REAL_CASEFILE: &str = "sce-build/tests/mutations/driver_fixture_citation.cases";
const A_REAL_STEM: &str = "driver_fixture_citation";

/// A second real casefile, holding several cases, for the questions that need
/// a record to name cases the casefile REALLY declares.
///
/// `holes` asks whether a case its record names is still in the casefile, so a
/// seeded record carrying invented labels would answer that question the wrong
/// way — every hole would read as a case that had vanished. This one is the
/// harness's own liveness casefile: it exists to keep `scripts/mutate` honest,
/// so it outlives the features around it.
const A_MULTI_CASE_CASEFILE: &str = "sce-build/tests/mutations/mutation_case_liveness.cases";
const A_MULTI_CASE_STEM: &str = "mutation_case_liveness";

#[test]
fn an_assertion_never_counts_as_a_measurement() {
    let ledger = tempdir().expect("temp ledger");

    let before = ask(ledger.path(), &["unjudged"]);
    assert!(
        before.lines().any(|line| line == A_REAL_STEM),
        "the fixture stem is not in the corpus any more — this test is asking \
         about a casefile that no longer exists"
    );

    let out = Command::new(repo_root().join("scripts/mutation-ledger"))
        .args([
            "assert",
            "--casefile",
            A_REAL_CASEFILE,
            "--note",
            "a round is claimed, its log is gone",
        ])
        .current_dir(repo_root())
        .env("SCE_MUTATION_LEDGER_DIR", ledger.path())
        .output()
        .expect("run mutation-ledger assert");
    assert!(out.status.success(), "the assertion was refused");

    // Recorded, and still unjudged. The note is worth keeping — it says a
    // round once passed, so re-running is a re-run and not a first look — but
    // it is a claim, and a claim that retired a casefile would be the whole
    // failure this ledger was built to end.
    let after = ask(ledger.path(), &["unjudged"]);
    assert_eq!(
        before, after,
        "an assertion moved the unjudged set, so a claim with no reading \
         behind it counts as a verdict"
    );
    assert!(
        ask(ledger.path(), &["claimed"])
            .lines()
            .any(|line| line.starts_with(A_REAL_STEM)),
        "the assertion was not kept anywhere a later session would find it"
    );
}

#[test]
fn a_measured_round_is_what_takes_a_casefile_off_the_unjudged_list() {
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");

    let before = ask(ledger.path(), &["unjudged"]);
    let before_count = before.lines().count();
    assert!(before.lines().any(|line| line == A_REAL_STEM));

    // The record a real round would leave, written through the same library a
    // real round writes through — the reading is simulated, the writing and
    // the accounting are not.
    with_library(
        home.path(),
        &format!(
            "export SCE_MUTATION_LEDGER_DIR={:?}; \
             rows=\"$(mktemp)\"; \
             mutation_ledger_begin {A_REAL_CASEFILE} \"$rows\"; \
             mutation_ledger_case CAUGHT 'a driver cites a fixture that was never real' '1/2 red'; \
             mutation_ledger_commit cargo 0 >/dev/null; \
             rm -f \"$rows\"",
            ledger.path().display().to_string()
        ),
    );

    let after = ask(ledger.path(), &["unjudged"]);
    assert_eq!(
        after.lines().count(),
        before_count - 1,
        "one round should retire exactly one casefile; the list went from \
         {before_count} to {}",
        after.lines().count()
    );
    assert!(
        !after.lines().any(|line| line == A_REAL_STEM),
        "the casefile the round judged is still listed as unjudged"
    );
    assert!(
        ask(ledger.path(), &["judged"])
            .lines()
            .any(|line| line.starts_with(A_REAL_STEM)),
        "the round's verdict is not readable back out of the ledger"
    );
}

#[test]
fn a_casefile_edited_since_its_round_is_reported_stale() {
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");

    // A blob hash no casefile has, standing for the version that was judged.
    // A verdict about a case that has since been rewritten is not a verdict
    // about the case in the tree, and a corpus reading 86/86 judged while a
    // third of it says something else is the quiet way this ledger goes bad.
    with_library(
        home.path(),
        &format!(
            "export SCE_MUTATION_LEDGER_DIR={:?}; \
             rows=\"$(mktemp)\"; \
             mutation_ledger_begin {A_REAL_CASEFILE} \"$rows\"; \
             MUTATION_LEDGER_BLOB=0000000000000000000000000000000000000000; \
             mutation_ledger_case CAUGHT 'a case as it was written then' '1/2 red'; \
             mutation_ledger_commit cargo 0 >/dev/null; \
             rm -f \"$rows\"",
            ledger.path().display().to_string()
        ),
    );

    assert!(
        ask(ledger.path(), &["stale"])
            .lines()
            .any(|line| line.starts_with(A_REAL_STEM)),
        "a casefile rewritten since the round that judged it reads as current"
    );
}

#[test]
fn a_case_the_round_did_not_catch_is_reported_as_a_hole() {
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");

    // One round with one of each verdict, written through the library a real
    // round writes through. Seeded rather than read off the real corpus,
    // which has no hole in it today: a witness that asked the real ledger
    // would assert against an empty sweep and stay green for exactly as long
    // as that held — the shape of test this repository has retired before.
    //
    // The LABELS are read off a casefile rather than invented, because `holes`
    // now asks whether a case its record names is still declared there.
    // Invented labels would every one of them come back VANISHED, and this
    // test would be measuring that instead of what it says it measures.
    let labels = labels_of(A_MULTI_CASE_CASEFILE);
    assert!(
        labels.len() >= 3,
        "{A_MULTI_CASE_CASEFILE} declares {} case(s); this test needs three \
         real labels to seed one verdict of each kind",
        labels.len()
    );
    with_library(
        home.path(),
        &format!(
            "export SCE_MUTATION_LEDGER_DIR={:?}; \
             rows=\"$(mktemp)\"; \
             mutation_ledger_begin {A_MULTI_CASE_CASEFILE} \"$rows\"; \
             mutation_ledger_case CAUGHT {:?} '1/2 red'; \
             mutation_ledger_case SURVIVED {:?} '0/2 red'; \
             mutation_ledger_case INCONCLUSIVE {:?} 'the mutated tree did not compile'; \
             mutation_ledger_commit cargo 1 >/dev/null; \
             rm -f \"$rows\"",
            ledger.path().display().to_string(),
            labels[0],
            labels[1],
            labels[2],
        ),
    );

    let holes = ask(ledger.path(), &["holes"]);
    for (verdict, label) in [("SURVIVED", &labels[1]), ("INCONCLUSIVE", &labels[2])] {
        assert!(
            holes.lines().any(|line| line.starts_with(A_MULTI_CASE_STEM)
                && line.contains(verdict)
                && line.contains(label.as_str())),
            "`holes` does not name the {verdict} case:\n{holes}"
        );
    }

    // The half that makes this a question rather than a listing: printing
    // every case the round measured would satisfy both assertions above.
    assert!(
        !holes.contains(labels[0].as_str()),
        "a CAUGHT case is reported as a hole:\n{holes}"
    );

    // And the casefile is still judged. A round that leaves a hole measured
    // the casefile all the same, and a reader that pushed it back onto the
    // unjudged list would make "nobody has looked" and "somebody looked and
    // it was bad" the same number again.
    assert!(
        !ask(ledger.path(), &["unjudged"])
            .lines()
            .any(|line| line == A_MULTI_CASE_STEM),
        "a round that left a hole put its casefile back on the unjudged list"
    );

    // The runner's own words, because they say how to repay it: a mutation
    // the compiler refused is not the same repair as one that survived.
    let detailed = ask(ledger.path(), &["holes", "--detail"]);
    assert!(
        detailed.contains("the mutated tree did not compile"),
        "`--detail` drops the reason the round gave:\n{detailed}"
    );

    // Counted in the summary too. Nobody runs a subcommand they have no
    // reason to suspect, which is why `stale` is counted there as well.
    let summary = ask(ledger.path(), &["status"]);
    assert!(
        summary.contains("2 case(s) not caught, in 1 casefile(s)"),
        "`status` does not count the cases a round did not catch:\n{summary}"
    );
}

#[test]
fn a_record_with_no_casefile_blob_is_reported_unverifiable_not_current() {
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");

    // What `import-log` writes: a console log does not say which casefile
    // version produced it, so the blob is recorded as `unknown` rather than
    // filled in from today's tree. The failure this guards is that `stale`
    // compares a RECORDED blob to the current one, so a record carrying none
    // is not "not stale" — the comparison never happens and silently passes.
    // Measured 2026-08-26: the corpus read `86/86 judged` and `0 stale` while
    // 37 of those 86 rested on recovered logs whose version nothing could
    // establish, and only a hand-written scan over the raw records saw it.
    with_library(
        home.path(),
        &format!(
            "export SCE_MUTATION_LEDGER_DIR={:?}; \
             rows=\"$(mktemp)\"; \
             mutation_ledger_begin {A_REAL_CASEFILE} \"$rows\"; \
             MUTATION_LEDGER_BLOB=unknown; \
             mutation_ledger_case CAUGHT 'a case some vanished log judged' '1/2 red'; \
             mutation_ledger_commit cargo 0 >/dev/null; \
             rm -f \"$rows\"",
            ledger.path().display().to_string()
        ),
    );

    assert!(
        ask(ledger.path(), &["unverifiable"])
            .lines()
            .any(|line| line.starts_with(A_REAL_STEM)),
        "a record carrying no casefile blob reads as one whose version is known"
    );

    // The point of keeping this apart from `stale`: asked of a record with no
    // blob, `stale` answers "no" to a question it never put. If this ever
    // starts listing the stem, the two questions have been folded together
    // and the quiet pass is back.
    assert!(
        !ask(ledger.path(), &["stale"])
            .lines()
            .any(|line| line.starts_with(A_REAL_STEM)),
        "`stale` claims to have examined a record that carries nothing to examine"
    );

    // Counted in the summary for the reason `stale` and `holes` are: a corpus
    // reading finished-and-verified when it is only the first is exactly what
    // nobody has a reason to run a subcommand about.
    let summary = ask(ledger.path(), &["status"]);
    assert!(
        summary.contains("1 unverifiable"),
        "`status` does not count records whose version cannot be checked:\n{summary}"
    );
}
