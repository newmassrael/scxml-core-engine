// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A kept console is bounded, and bounding it does not move a verdict.
//
// `SCE_MUTATION_REPORT_DIR` exists because `$WORK` is a mktemp directory the
// EXIT trap removes, so a verdict the harness cannot explain from its own
// summary is unreadable an hour later — it is what told G7's `ran 0 tests`
// apart from a timeout. But CTest holds a test's whole output in memory and
// prints all of it on failure, so one red run of a mutant that loops while
// logging is not a log: measured, `undecodable_payload_is_reported_ctest` left
// 678 MB per eleven-case round, 676 MB of it console at ~97 MB per red run,
// against 2.1 MB of JUnit for the entire round. On `/tmp` that is rude; aimed
// at a persistent directory it fills a shared disk in a few rounds.
//
// The cap is therefore on the COPY, and this file's job is to hold that
// distinction still, because getting it wrong is silent in the worst direction.
// The console a verdict is drawn from is `$WORK/ctest-console.txt`, handed
// unabridged to `mutation_failures_from_gtest`; the copy under the report
// directory has no reader in this repository at all — measured by grep over
// every `.sh`, `.py`, `.rs`, `.yml` and `.md`, and `scripts/mutation-ledger`'s
// "console log" is the round's own stdout, a different artefact. If a cap ever
// reached the parser it would quietly shorten the `red:` list, which is the one
// place a CAUGHT verdict says whether the tests that turned red are the ones
// that own the clause — a mutation caught only by an unrelated suite is a case
// aimed at nothing, and the names are where that shows.
//
// So the fixture buries a failing test's NAME IN THE MIDDLE of the flood, which
// is the one place a two-sided cap does not reach. The assertion that the name
// survives is what makes this more than a size check — and it is measured, not
// assumed: with the cap wrongly moved to the verdict side, the round still
// reported CAUGHT and the record named one of the two tests that turned red.
//
// What the cap does cost is real and worth stating: a kept console loses the
// failure TEXT of anything that failed in the middle. What it does not cost is
// knowing WHICH tests failed — that is in the JUnit document, which is left
// uncapped at ~2 MB for a whole round, and in the round's own `red:` line and
// ledger record, both drawn from the full console.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

/// The ceiling the harness defaults to, spelled here so the assertions below
/// are about a number this test states rather than one it reads back out of the
/// thing it is measuring.
const CAP: u64 = 2 * 1024 * 1024;

/// Named markers at each end of the flood. The point of keeping both ends is
/// that a reader's diagnosis lives there; asserting on the markers is how
/// "both ends kept" stops being a claim in a comment.
const HEAD_MARK: &str = "SCE_CONSOLE_HEAD_MARKER";
const TAIL_MARK: &str = "SCE_CONSOLE_TAIL_MARKER";
const RED_NAME: &str = "TheSuite.TheCaseThatOwnsTheClause";
/// The name a two-sided cap would drop. Its survival is what says the verdict
/// was read from the whole console and not from the copy.
const MIDDLE_NAME: &str = "TheSuite.TheCaseInTheMiddle";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

const SUBJECT: &str = r#"#include "subject.h"

int is_sound(void) {
    return 1;
}
"#;

const HEADER: &str = r#"#ifndef SUBJECT_H
#define SUBJECT_H
int is_sound(void);
#endif
"#;

/// The oracle. Sound: a line and a zero exit, so the baseline's console is
/// small and the floor below has something to measure. Mutated: a head marker,
/// flood, a failure IN THE MIDDLE, more flood, then the closing failure and its
/// summary repeat.
///
/// ⚠ The middle failure is the discriminating part, and the first draft of this
/// fixture did not have it. It put every name at the tail — and a two-sided cap
/// keeps the tail, so applying the cap to the VERDICT side (the exact mistake
/// this file exists to catch) still passed. Measured: with that wrong edit in
/// place the test went green. A failure in the middle is also the ordinary
/// shape, not a contrived one: gtest prints each failure where it happens, so
/// any red run over more than one test has names before the end.
const ORACLE: &str = r#"#include <stdio.h>
#include "subject.h"

static void flood(int from, int to) {
    for (int i = from; i < to; ++i) {
        printf("the raiser answered a refusal by leaving it queued %d\n", i);
    }
}

int main(void) {
    if (is_sound()) {
        printf("[       OK ] TheSuite.TheCaseThatOwnsTheClause (0 ms)\n");
        return 0;
    }
    printf("SCE_CONSOLE_HEAD_MARKER\n");
    flood(0, 100000);
    printf("[  FAILED  ] TheSuite.TheCaseInTheMiddle (1 ms)\n");
    flood(100000, 200000);
    /* gtest prints a failure twice: where it happens and in the summary. Both
       shapes are here because the parser de-duplicates them, and a fixture
       that emitted only one would not exercise that. */
    printf("[  FAILED  ] TheSuite.TheCaseThatOwnsTheClause (1 ms)\n");
    printf("[  FAILED  ] 2 tests, listed below:\n");
    printf("[  FAILED  ] TheSuite.TheCaseThatOwnsTheClause\n");
    printf("SCE_CONSOLE_TAIL_MARKER\n");
    fflush(stdout);
    return 1;
}
"#;

const CMAKELISTS: &str = r#"cmake_minimum_required(VERSION 3.16)
project(g8_kept_console C)
enable_testing()
add_executable(g8_oracle oracle.c subject.c)
target_include_directories(g8_oracle PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
add_test(NAME g8_oracle COMMAND g8_oracle)
set_tests_properties(g8_oracle PROPERTIES
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR} TIMEOUT 60)
"#;

const CASES: &str = r#"
mutation_case "the soundness check stops being sound" <<'PY'
edit(TARGET, "    return 1;", "    return 0;")
PY
"#;

/// Inside the repository, under a gitignored path: `scripts/mutate` derives its
/// root from `git rev-parse --show-toplevel` and its ctest runner refuses a
/// test artifact whose real path is outside that root, which a symlinked
/// `target/` is enough to fail.
struct Project {
    root: PathBuf,
    casefile: PathBuf,
    reports: PathBuf,
    _ledger: TempDir,
    ledger: PathBuf,
    _home: TempDir,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("run {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn project(name: &str) -> Project {
    let root = repo_root().join("tmp").join(format!("g8-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the fixture directory");

    fs::write(root.join("subject.c"), SUBJECT).expect("write subject.c");
    fs::write(root.join("subject.h"), HEADER).expect("write subject.h");
    fs::write(root.join("oracle.c"), ORACLE).expect("write oracle.c");
    fs::write(root.join("CMakeLists.txt"), CMAKELISTS).expect("write CMakeLists.txt");

    let build = root.join("build");
    run(
        Command::new("cmake")
            .arg("-S")
            .arg(&root)
            .arg("-B")
            .arg(&build)
            .arg("-DCMAKE_BUILD_TYPE=Release"),
        "cmake configure",
    );
    run(
        Command::new("cmake").arg("--build").arg(&build),
        "cmake build",
    );

    let rel = |p: &str| format!("tmp/g8-{name}/{p}");
    let casefile = root.join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "mutation_ctest --test-dir {build} -R g8_oracle\n\
             mutation_targets {subject}\n\
             mutation_oracles {oracle}\n{CASES}",
            build = build.display(),
            subject = rel("subject.c"),
            oracle = rel("oracle.c"),
        )
        .replace("TARGET", &format!("{:?}", rel("subject.c"))),
    )
    .expect("write the casefile");

    let reports = root.join("reports");
    let ledger = tempdir().expect("temp ledger");
    Project {
        root,
        casefile,
        reports,
        ledger: ledger.path().to_path_buf(),
        _ledger: ledger,
        _home: tempdir().expect("temp home"),
    }
}

impl Project {
    /// Run a full round with the report directory set. `cap` overrides the
    /// harness default when given, so a test can say which ceiling it means.
    fn round(&self, cap: Option<u64>) -> (bool, String) {
        let mut cmd = Command::new(repo_root().join("scripts/mutate"));
        cmd.arg(&self.casefile)
            .current_dir(repo_root())
            .env("HOME", self._home.path())
            .env("SCE_MUTATION_LEDGER_DIR", &self.ledger)
            .env("SCE_MUTATION_REPORT_DIR", &self.reports)
            .env("SCE_BUILD_JOBS", "2");
        if let Some(cap) = cap {
            cmd.env("MUTATION_KEPT_CONSOLE_MAX_BYTES", cap.to_string());
        }
        let out = cmd.output().expect("run scripts/mutate");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.success(), text)
    }

    /// The kept consoles, as (name, size, contents).
    fn consoles(&self) -> Vec<(String, u64, String)> {
        let mut found: Vec<_> = fs::read_dir(&self.reports)
            .expect("read the report directory")
            .map(|e| e.expect("a directory entry").path())
            .filter(|p| p.to_string_lossy().ends_with(".console.txt"))
            .map(|p| {
                let name = p
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                let size = fs::metadata(&p).expect("stat the console").len();
                let body = fs::read_to_string(&p).unwrap_or_default();
                (name, size, body)
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }

    fn record(&self) -> String {
        let mut found: Vec<PathBuf> = fs::read_dir(&self.ledger)
            .expect("read the ledger directory")
            .map(|e| e.expect("a directory entry").path())
            .collect();
        assert_eq!(
            found.len(),
            1,
            "expected one ledger record, found {found:?}"
        );
        fs::read_to_string(found.pop().expect("one record")).expect("read the record")
    }
}

#[test]
fn a_kept_console_is_capped_at_both_ends_and_says_what_it_dropped() {
    let p = project("capped");
    let (ok, output) = p.round(None);
    assert!(ok, "the round did not end cleanly:\n{output}");

    let consoles = p.consoles();
    // Two runs: the baseline and the one mutated case. The restore does not run
    // the suite — it only rebuilds — so a third would mean the round took a
    // path this test is not describing.
    assert_eq!(
        consoles.len(),
        2,
        "expected a kept console per run, got {:?}",
        consoles.iter().map(|c| &c.0).collect::<Vec<_>>()
    );

    // Slack for the marker paragraph the truncation writes between the halves.
    let ceiling = CAP + 4096;
    for (name, size, _) in &consoles {
        assert!(
            *size <= ceiling,
            "{name} is {size} bytes, past the {ceiling} the cap allows"
        );
    }

    let (name, size, body) = consoles
        .iter()
        .find(|(_, _, body)| body.contains("were dropped here"))
        .unwrap_or_else(|| {
            panic!(
                "no kept console was truncated, so this fixture did not flood one: {:?}",
                consoles.iter().map(|(n, s, _)| (n, s)).collect::<Vec<_>>()
            )
        });

    // Both ends, which is the whole reason for a two-sided cap: the head has
    // the run's beginning and the tail has the verdict lines.
    assert!(
        body.contains(HEAD_MARK),
        "{name} lost the head of the console it kept"
    );
    assert!(
        body.contains(TAIL_MARK),
        "{name} lost the tail, where the verdict lines are"
    );
    assert!(
        body.find(HEAD_MARK) < body.find("were dropped here")
            && body.find("were dropped here") < body.find(TAIL_MARK),
        "{name} does not read head, gap, tail in that order"
    );
    // And the gap says how much, in bytes, rather than only that there was one:
    // a reader who cannot tell a truncated console from a short one reads a
    // missing failure as a failure that did not happen.
    assert!(
        body.contains("the kept ceiling is"),
        "{name} was truncated without naming the ceiling:\n{}",
        &body[..body.len().min(400)]
    );
    assert!(
        *size > 1024,
        "{name} is suspiciously small for a capped copy"
    );
}

#[test]
fn capping_the_copy_does_not_shorten_the_verdict() {
    // The assertion this whole change turns on. The fixture buries a failing
    // test's name in the MIDDLE of nine megabytes of flood, which is the one
    // place a two-sided cap does not reach. It survives only if the parser read
    // the whole console. A cap applied to the verdict side instead of the copy
    // would drop it and the round would still report CAUGHT — naming one of the
    // two tests that turned red, which reads like a narrower case than it is.
    let p = project("verdict");
    let (ok, output) = p.round(None);
    assert!(ok, "the round did not end cleanly:\n{output}");

    let record = p.record();
    assert!(
        record.contains(MIDDLE_NAME),
        "the verdict lost the name buried in the middle of the flood, so it was \
         read from a truncated console:\n{record}"
    );
    assert!(
        record.contains(RED_NAME),
        "the verdict lost the name at the end of the flood:\n{record}"
    );
    // De-duplicated, not counted twice: the fixture emits gtest's failure line
    // and its summary repeat, and the parser owes exactly one name.
    assert_eq!(
        record.matches(RED_NAME).count(),
        1,
        "the name should appear once in the record:\n{record}"
    );
    // The console printed to the terminal shows at most three names, so the
    // round's own line is checked for one of them and the record for both.
    assert!(
        output.contains("red: "),
        "the round named nothing red at all:\n{output}"
    );
}

#[test]
fn a_console_under_the_cap_is_kept_whole() {
    // The floor. Without it the cap could be doing nothing but truncating
    // everything, and every assertion above would still pass — a report
    // directory of uniformly mangled files reads the same as a bounded one.
    let p = project("whole");
    // A ceiling above the flood, so the same fixture that gets truncated in the
    // test above is kept entire here. Same round, same console, different rule.
    let (ok, output) = p.round(Some(64 * 1024 * 1024));
    assert!(ok, "the round did not end cleanly:\n{output}");

    let consoles = p.consoles();
    for (name, _, body) in &consoles {
        assert!(
            !body.contains("were dropped here"),
            "{name} was truncated under a ceiling that fits it whole"
        );
    }
    let flooded = consoles
        .iter()
        .find(|(_, size, _)| *size > CAP)
        .unwrap_or_else(|| panic!("nothing exceeded {CAP} bytes, so the floor proves nothing"));
    assert!(
        flooded.2.contains(HEAD_MARK) && flooded.2.contains(TAIL_MARK),
        "{} is missing an end it was supposed to keep whole",
        flooded.0
    );
}
