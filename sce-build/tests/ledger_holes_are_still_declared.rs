// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A hole is a case that still exists.
//
// A ledger record is a reading of a casefile AS IT WAS. When the casefile
// changes, cases in that record can stop existing — and `holes` walked the
// newest record without ever asking whether they still did.
//
// Measured 2026-08-30: the corpus reported seven holes and FOUR of them were
// gone. `lua_engine_reads_ecmascript` had been judged holding eight cases;
// retiring the rewriter cut it to three, and the four survivors moved with the
// member surface to `the_frontend_now_owns_the_member_surface`, where a cargo
// round CAUGHT every one. The debt list named four repaid debts, which is how
// a debt list stops being read. `stale` knew that casefile had moved and said
// so in its own subcommand; nothing carried the fact into this one.
//
// Both directions are measured here, because only one of them is loud.
// Reporting too FEW vanished cases leaves repaid debts on the list — visible,
// annoying, eventually noticed. Reporting too MANY empties the list while the
// debts stand, and nothing says so: a label parser that drifted off the
// casefile format would report every hole in the corpus as vanished. That is
// why an unreadable label makes the question UNANSWERABLE rather than false,
// and why that guard gets a test of its own.
//
// ⚠ Separate from `mutation_ledger` for one reason, and it is not subject
// matter: a mutation round cannot use that suite as an oracle. One of its
// tests drives `scripts/mutate --check`, which calls `cargo metadata`, and
// inside a cargo round that comes back `cargo metadata failed — the selector
// cannot be checked` — the baseline is red before a case is applied. A suite
// that cannot be a baseline cannot be an oracle, so the contract that needs
// mutation cases lives here.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

mod common;
use common::ledger::{ask, ask_with_corpus, labels_of, repo_root, with_library};

/// A casefile the corpus really holds, with several cases in it.
///
/// The harness's own liveness casefile: it exists to keep `scripts/mutate`
/// honest, so it outlives the features around it.
const A_MULTI_CASE_CASEFILE: &str = "sce-build/tests/mutations/mutation_case_liveness.cases";
const A_MULTI_CASE_STEM: &str = "mutation_case_liveness";
/// A second real casefile, to borrow a label from — a case that exists in the
/// corpus but not in the one the record is about.
const ANOTHER_CASEFILE: &str = "sce-build/tests/mutations/driver_fixture_citation.cases";
const ANOTHER_STEM: &str = "driver_fixture_citation";

#[test]
fn a_case_the_casefile_no_longer_declares_is_not_reported_as_open_work() {
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");

    let here = labels_of(A_MULTI_CASE_CASEFILE);
    let elsewhere = labels_of(ANOTHER_CASEFILE);
    assert!(
        !here.is_empty() && !elsewhere.is_empty(),
        "both fixture casefiles must declare at least one case: {} and {}",
        here.len(),
        elsewhere.len()
    );
    // The premise this test rests on, asserted rather than assumed: the
    // borrowed label must NOT be one the seeded casefile also declares, or
    // "vanished" would be the wrong answer and the assertion would measure
    // nothing.
    assert!(
        !here.contains(&elsewhere[0]),
        "the two fixture casefiles share the label {:?}; this test needs one \
         that belongs to the other file alone",
        elsewhere[0]
    );
    let never_declared = "a case no casefile in this corpus declares";
    assert!(
        !here.contains(&never_declared.to_string()),
        "the invented label is no longer invented"
    );

    with_library(
        home.path(),
        &format!(
            "export SCE_MUTATION_LEDGER_DIR={:?}; \
             rows=\"$(mktemp)\"; \
             mutation_ledger_begin {A_MULTI_CASE_CASEFILE} \"$rows\"; \
             mutation_ledger_case SURVIVED {:?} '0/2 red'; \
             mutation_ledger_case SURVIVED {:?} '0/2 red'; \
             mutation_ledger_case SURVIVED {:?} '0/2 red'; \
             mutation_ledger_commit cargo 1 >/dev/null; \
             rm -f \"$rows\"",
            ledger.path().display().to_string(),
            here[0],
            never_declared,
            elsewhere[0],
        ),
    );

    let holes = ask(ledger.path(), &["holes"]);

    // The case that is still there stays a hole. Without this the repair could
    // be "report nothing", which empties the debt list by deleting it.
    assert!(
        holes.lines().any(|line| line.contains(here[0].as_str())
            && line.contains("SURVIVED")
            && !line.contains("VANISHED")),
        "a case the casefile still declares stopped being a hole:\n{holes}"
    );

    // The one nothing declares: named as vanished, its old verdict kept, and
    // the reader told that a round is what settles it.
    assert!(
        holes.lines().any(|line| line.contains(never_declared)
            && line.contains("VANISHED")
            && line.contains("(was SURVIVED)")),
        "a case no casefile declares is still reported as open work:\n{holes}"
    );
    assert!(
        holes.contains("no casefile declares this label"),
        "`holes` does not say the label could not be found anywhere:\n{holes}"
    );

    // The one that moved: named, and followed.
    assert!(
        holes
            .lines()
            .any(|line| line.contains(elsewhere[0].as_str()) && line.contains("VANISHED")),
        "a case that moved to another casefile is still reported as open \
         work:\n{holes}"
    );
    assert!(
        holes.contains(&format!("now declared in: {ANOTHER_STEM}")),
        "`holes` does not name the casefile that declares the moved case \
         now:\n{holes}"
    );

    // And the summary splits them, because they are different work: an open
    // case needs a repair, a vanished one needs a round.
    let summary = ask(ledger.path(), &["status"]);
    assert!(
        summary.contains("1 case(s) not caught, in 1 casefile(s)"),
        "`status` counts a vanished case as open work:\n{summary}"
    );
    assert!(
        summary.contains("2 case(s) the record names but the casefile no longer declares"),
        "`status` does not count the cases whose record outlived them:\n{summary}"
    );
    // The stem is still judged: a record older than its casefile is still a
    // reading, and pushing it back onto the unjudged list would make "nobody
    // looked" and "somebody looked at an earlier version" the same number.
    assert!(
        !ask(ledger.path(), &["unjudged"])
            .lines()
            .any(|line| line == A_MULTI_CASE_STEM),
        "a vanished case put its casefile back on the unjudged list"
    );
}

#[test]
fn a_casefile_whose_labels_cannot_be_read_keeps_its_holes() {
    // The silent direction. `holes` decides a case has vanished by not finding
    // its label — so a parser that drifted off the casefile format would
    // report EVERY hole in the corpus as vanished, and the debt list would
    // empty itself while the debts stood.
    //
    // Measured while building this: written as `labels.split(FS) if labels
    // else []`, a casefile whose single label could not be read came back with
    // NO labels rather than one empty one, and its hole was reported VANISHED.
    // An empty string and an empty list are not the same fact.
    let home = tempdir().expect("temp home");
    let ledger = tempdir().expect("temp ledger");
    let corpus = tempdir().expect("temp corpus");

    // One case, declared with an unquoted label: `mutation_case` is there, so
    // it counts as a case, and no label can be read from it.
    let casefile = corpus.path().join("an_unreadable_label.cases");
    fs::write(
        &casefile,
        "mutation_tests -p sce-build --test nothing\n\
         mutation_targets scripts/mutation-ledger\n\n\
         mutation_case unquoted_label_the_reader_cannot_take <<'PY'\n\
         edit(\"scripts/mutation-ledger\", \"a\", \"b\")\n\
         PY\n",
    )
    .expect("write the fixture casefile");

    with_library(
        home.path(),
        &format!(
            "export SCE_MUTATION_LEDGER_DIR={:?}; \
             export SCE_MUTATION_CORPUS_DIR={:?}; \
             rows=\"$(mktemp)\"; \
             mutation_ledger_begin {:?} \"$rows\"; \
             mutation_ledger_case SURVIVED 'a case whose label the reader cannot take' '0/2 red'; \
             mutation_ledger_commit cargo 1 >/dev/null; \
             rm -f \"$rows\"",
            ledger.path().display().to_string(),
            corpus.path().display().to_string(),
            casefile.display().to_string(),
        ),
    );

    let holes = ask_with_corpus(ledger.path(), Some(corpus.path()), &["holes"]);
    assert!(
        holes.contains("SURVIVED"),
        "a hole in a casefile whose labels cannot be read stopped being a \
         hole:\n{holes}"
    );
    assert!(
        !holes.contains("VANISHED"),
        "an unreadable label was read as a case that no longer exists, which \
         is how this debt list would empty itself:\n{holes}"
    );
    assert!(
        holes.contains("labels could not all be read"),
        "`holes` does not say why it declined to judge whether the case still \
         exists:\n{holes}"
    );
}

#[test]
fn the_corpus_this_repository_ships_has_every_label_readable() {
    // The floor under the guard above. That guard is correct and, on this
    // tree, unreachable — which is exactly the state in which a rule quietly
    // stops being true. This measures the premise directly: if a casefile ever
    // arrives whose label the reader cannot take, this says so on the day it
    // lands rather than leaving `holes` to report "unknown" forever.
    let corpus = repo_root().join("sce-build/tests/mutations");
    let mut casefiles = 0usize;
    let mut cases = 0usize;
    let mut unreadable = Vec::new();
    for entry in fs::read_dir(&corpus).expect("read the corpus directory") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|e| e != "cases") {
            continue;
        }
        casefiles += 1;
        let body = fs::read_to_string(&path).expect("read a casefile");
        let declared = body
            .lines()
            .filter(|line| line.split(' ').next() == Some("mutation_case"))
            .count();
        cases += declared;
        let readable = labels_of(
            path.strip_prefix(repo_root())
                .expect("a path under the repository")
                .to_str()
                .expect("utf-8 path"),
        )
        .len();
        if readable != declared {
            unreadable.push(format!(
                "{}: {declared} case(s), {readable} label(s) readable",
                path.display()
            ));
        }
    }
    assert!(
        casefiles >= 50 && cases >= 300,
        "the corpus scan found {casefiles} casefile(s) and {cases} case(s) — \
         it has lost its subject"
    );
    assert!(
        unreadable.is_empty(),
        "some casefiles declare cases whose labels the ledger cannot read, so \
         `holes` cannot say whether they still exist:\n{}",
        unreadable.join("\n")
    );
}

#[test]
fn the_shipped_corpus_answers_the_question_at_all() {
    // The end-to-end floor: the real corpus and the real ledger, through the
    // real tool. A `holes` that crashed on the shipped tree would fail every
    // test above only if they happened to reach it, and they run against
    // fixtures.
    let out = Command::new(repo_root().join("scripts/mutation-ledger"))
        .arg("status")
        .current_dir(repo_root())
        .output()
        .expect("run mutation-ledger status");
    assert!(
        out.status.success(),
        "`status` failed over the shipped corpus:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let summary = String::from_utf8_lossy(&out.stdout);
    assert!(
        summary.contains("case(s) the record names but the casefile no longer declares"),
        "`status` does not report the vanished count at all:\n{summary}"
    );
}
