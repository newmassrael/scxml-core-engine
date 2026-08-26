// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A build that is not byte-reproducible does not end the casefile.
//
// After every case `scripts/mutate` restores the tree, rebuilds, and asks
// whether it is looking at the baseline again. The question is right — a later
// case measured against a moved baseline reports on the wrong tree — but the
// answer used to be taken from the BINARIES, and a binary is not the harness's
// to promise. Measured on `unseen_event_is_reported_ctest`: the round stopped
// at its second case with `restore did not reproduce the baseline binaries`,
// six of eight cases were never measured, no ledger record was written at all,
// and the casefile read as a red round. A second round over the identical
// casefile ran all eight, so what cost two-thirds of a casefile was
// intermittent and had nothing to do with the code under test. One of those six
// then sat unjudged for weeks.
//
// The tree this drives makes that condition permanent instead of intermittent:
// its build generates a serial header before every compile, so two builds of
// byte-identical sources produce executables whose `.rodata` differs. That is
// the cause injected rather than imitated — a build that embeds a build number,
// a timestamp or a `git describe` is not reproducible in exactly this way, and
// none of what it embeds is a fact about the code under test.
//
// It has to be CONTENT and not a nonce, because `mutation_artifact_hash`
// already normalises the nonce class away: a `--strip-debug` plus a dropped
// `.note.gnu.build-id` is what stopped split DWARF's per-invocation `DWO ID`
// from reading as a botched restore (measured 2026-08-14). That normalisation
// is the right instrument and stays; what it cannot do is make a build that
// embeds something new each time look like a build that does not, and G6 fired
// with it already in place.
//
// Three things are measured through it, and each one is a way the repair could
// be wrong rather than absent:
//
//   - the round REACHES ITS END and judges every case. This is the repair.
//   - it SAYS the baseline was retaken. Recovering silently would trade a
//     round that dies for a round whose reader cannot tell that the thing it
//     compares against moved underneath it.
//   - a baseline that CANNOT be earned back still stops the round — and names
//     the cases it never reached, in the console and in the ledger record.
//     Continuing there would measure later cases against a tree that really
//     has moved, which is the failure the original check existed to prevent.
//
// The real `scripts/mutate` is run, over a real CMake/CTest project, because
// the code under test is the case loop's own tail and a copy of it here would
// be the second spelling of the thing that broke.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// A CMake/CTest project inside the repository, under a gitignored path.
///
/// Inside, and not in `/tmp`, for two reasons the harness states itself:
/// `scripts/mutate` derives its root from `git rev-parse --show-toplevel` and
/// sources its libraries from there, and its ctest runner refuses a test
/// artifact whose real path is outside that root — a symlinked build directory
/// is enough to fail that, so the fixture lives where the check can see it.
struct Project {
    root: PathBuf,
    casefile: PathBuf,
    _ledger: TempDir,
    ledger: PathBuf,
    _home: TempDir,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The three mutable sites and the assertions that own them. Deliberately
/// plain: every case below is CAUGHT on a tree that works, so a round that
/// ends any other way is the harness's doing and not the fixture's.
const SUBJECT: &str = r#"#include "subject.h"

int doubled(int x) {
    return x * 2;
}

int clamped(int x) {
    return x < 0 ? 0 : x;
}

int counted(const int *xs, int n) {
    int total = 0;
    for (int i = 0; i < n; ++i) {
        total += xs[i];
    }
    return total;
}
"#;

const HEADER: &str = r#"#ifndef SUBJECT_H
#define SUBJECT_H
int doubled(int x);
int clamped(int x);
int counted(const int *xs, int n);
#endif
"#;

/// The oracle. It also refuses to pass while the marker file exists, which is
/// how the third test below arranges a tree that is genuinely no longer the
/// baseline — a suite that went red for a reason the harness cannot restore.
const ORACLE: &str = r#"#include <stdio.h>
#include "subject.h"
#include "serial.h"

/* What makes this executable differ from the last one built out of the same
   sources. Kept in `.rodata` and read at run time so no optimiser can fold it
   away: a difference the linker drops is not the condition under test. */
const char *const build_serial = BUILD_SERIAL;

int main(void) {
    FILE *marker = fopen("moved", "r");
    if (marker != NULL) {
        fclose(marker);
        fprintf(stderr, "the tree is no longer the baseline\n");
        return 1;
    }
    if (build_serial[0] == '\0') {
        fprintf(stderr, "serial\n");
        return 1;
    }
    const int xs[3] = {1, 2, 3};
    if (doubled(3) != 6) {
        fprintf(stderr, "doubled\n");
        return 1;
    }
    if (clamped(-4) != 0) {
        fprintf(stderr, "clamped\n");
        return 1;
    }
    if (counted(xs, 3) != 6) {
        fprintf(stderr, "counted\n");
        return 1;
    }
    return 0;
}
"#;

/// The step that makes the artifact move: a header regenerated before every
/// compile, with a value that is new each time.
///
/// `BREAK_AT` of 0 never writes the marker; a positive value writes it once the
/// serial reaches it, which is a tree the harness cannot put back by restoring
/// files — the suite is red for a reason no snapshot holds.
const STAMP: &str = r##"set(serial 0)
if(EXISTS "${SERIAL_FILE}")
  file(READ "${SERIAL_FILE}" serial)
endif()
math(EXPR serial "${serial} + 1")
file(WRITE "${SERIAL_FILE}" "${serial}")
file(WRITE "${HEADER}" "#define BUILD_SERIAL \"sce build ${serial}\"\n")
if(BREAK_AT GREATER 0 AND NOT serial LESS BREAK_AT)
  file(WRITE "${MARKER}" "moved\n")
endif()
"##;

/// `moving` decides whether the build embeds something new each time. A fixture
/// that does NOT is how the ordinary path gets measured — 86 casefiles take it,
/// and a repair that only ever runs on the exceptional one would be untested
/// where it matters most.
fn cmakelists(break_at: u32, moving: bool) -> String {
    let serial = if moving {
        // A target with a command is always out of date, so the serial header
        // is written afresh on every build and the oracle's translation unit
        // recompiles against it. Two builds of identical sources therefore
        // differ in `.rodata`, which is the condition under test and the one
        // part of a binary that normalising away debug identifiers cannot make
        // stable.
        format!(
            r#"add_custom_target(g6_serial ALL
    BYPRODUCTS ${{CMAKE_BINARY_DIR}}/serial.h
    COMMAND ${{CMAKE_COMMAND}}
            -DSERIAL_FILE=${{CMAKE_BINARY_DIR}}/serial
            -DHEADER=${{CMAKE_BINARY_DIR}}/serial.h
            -DMARKER=${{CMAKE_BINARY_DIR}}/moved
            -DBREAK_AT={break_at}
            -P ${{CMAKE_CURRENT_SOURCE_DIR}}/stamp.cmake)
add_dependencies(g6_oracle g6_serial)
"#
        )
    } else {
        // Written once, at configure time, so every build of the same sources
        // produces the same binary.
        "file(WRITE ${CMAKE_BINARY_DIR}/serial.h \
         \"#define BUILD_SERIAL \\\"fixed\\\"\\n\")\n"
            .to_string()
    };
    format!(
        r#"cmake_minimum_required(VERSION 3.16)
project(g6_moved_baseline C)
enable_testing()
add_executable(g6_oracle oracle.c subject.c)
target_include_directories(g6_oracle PRIVATE
    ${{CMAKE_CURRENT_SOURCE_DIR}} ${{CMAKE_BINARY_DIR}})
{serial}add_test(NAME g6_oracle COMMAND g6_oracle)
set_tests_properties(g6_oracle PROPERTIES
    WORKING_DIRECTORY ${{CMAKE_BINARY_DIR}} TIMEOUT 30)
"#
    )
}

const CASES: &str = r#"
mutation_case "doubling is not adding" <<'PY'
edit(TARGET, "    return x * 2;", "    return x + 2;")
PY

mutation_case "the clamp lets a negative through" <<'PY'
edit(TARGET, "    return x < 0 ? 0 : x;", "    return x;")
PY

mutation_case "the total never accumulates" <<'PY'
edit(TARGET, "        total += xs[i];", "        total += 0;")
PY
"#;

/// The labels above, in order, as the harness prints and records them.
const LABELS: [&str; 3] = [
    "doubling is not adding",
    "the clamp lets a negative through",
    "the total never accumulates",
];

fn run(cmd: &mut Command, what: &str) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("run {what}: {e}"));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{what} failed:\n{text}");
    text
}

fn project(name: &str, break_at: u32, moving: bool) -> Project {
    let root = repo_root().join("tmp").join(format!("g6-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the fixture directory");

    fs::write(root.join("subject.c"), SUBJECT).expect("write subject.c");
    fs::write(root.join("subject.h"), HEADER).expect("write subject.h");
    fs::write(root.join("oracle.c"), ORACLE).expect("write oracle.c");
    fs::write(root.join("stamp.cmake"), STAMP).expect("write stamp.cmake");
    fs::write(root.join("CMakeLists.txt"), cmakelists(break_at, moving))
        .expect("write CMakeLists.txt");

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
    // The first link, so the round's own baseline build is a no-op and the
    // serial it hashes is settled before the first case.
    run(
        Command::new("cmake").arg("--build").arg(&build),
        "cmake build",
    );

    // Paths in a casefile are read from the repository root, which is where
    // `scripts/mutate` stands.
    let rel = |p: &str| format!("tmp/g6-{name}/{p}");
    let casefile = root.join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "mutation_ctest --test-dir {build} -R g6_oracle\n\
             mutation_targets {subject}\n\
             mutation_oracles {oracle}\n{CASES}",
            build = build.display(),
            subject = rel("subject.c"),
            oracle = rel("oracle.c"),
        )
        .replace("TARGET", &format!("{:?}", rel("subject.c"))),
    )
    .expect("write the casefile");

    let ledger = tempdir().expect("temp ledger");
    Project {
        root,
        casefile,
        ledger: ledger.path().to_path_buf(),
        _ledger: ledger,
        _home: tempdir().expect("temp home"),
    }
}

impl Project {
    /// Run a full round. `harness` is the script to run it with, so a control
    /// can drive the pre-repair one out of git history.
    fn round_with(&self, harness: &Path) -> (bool, String) {
        let out = Command::new(harness)
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("HOME", self._home.path())
            .env("SCE_MUTATION_LEDGER_DIR", &self.ledger)
            .env("SCE_BUILD_JOBS", "2")
            .output()
            .expect("run scripts/mutate");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.success(), text)
    }

    fn round(&self) -> (bool, String) {
        self.round_with(&repo_root().join("scripts/mutate"))
    }

    /// The one record the round wrote, as text. A round that wrote none is a
    /// failure of its own: a stopped round used to leave nothing behind.
    fn record(&self) -> String {
        let mut found: Vec<PathBuf> = fs::read_dir(&self.ledger)
            .expect("read the ledger directory")
            .map(|e| e.expect("a directory entry").path())
            .collect();
        assert_eq!(
            found.len(),
            1,
            "expected exactly one ledger record, found {found:?}"
        );
        fs::read_to_string(found.pop().expect("one record")).expect("read the record")
    }
}

#[test]
fn a_build_that_is_not_reproducible_does_not_end_the_casefile() {
    let p = project("reaches-its-end", 0, true);
    let (ok, output) = p.round();

    // Every case, in the round's own words. Counting them is not enough: the
    // failure this is about stops PARTWAY, so what matters is which labels
    // reached a verdict and not how many verdict lines were printed.
    for label in LABELS {
        assert!(
            output.contains(&format!("CAUGHT        {label}")),
            "the round did not judge {label:?}:\n{output}"
        );
    }
    assert!(
        !output.contains("not judged:"),
        "the round left a case unmeasured on a tree it could restore:\n{output}"
    );
    assert!(
        ok,
        "a round whose every case was caught did not end cleanly:\n{output}"
    );

    // And it said so. A baseline that moves under the cases is a fact about
    // the round, and a reader who is not told cannot weigh what follows it.
    assert!(
        output.contains("the baseline is being retaken"),
        "the baseline moved three times and the round never said so:\n{output}"
    );
    assert!(
        output.contains("baseline retaken:"),
        "the round retook the baseline without reporting what it earned:\n{output}"
    );

    let record = p.record();
    assert!(
        record.contains("\"rc\": 0"),
        "the record does not say the round reached its end:\n{record}"
    );
    for label in LABELS {
        assert!(
            record.contains(label),
            "the record is missing {label:?}:\n{record}"
        );
    }
}

#[test]
fn a_build_that_is_reproducible_never_retakes_the_baseline() {
    // The path every casefile in the corpus takes. The repair reorders the two
    // questions the post-case check asks and adds a recovery to one of them, so
    // the ordinary answer — nothing moved, carry on — has to be measured too:
    // a harness that retook its baseline after every case would be hiding the
    // very drift this check exists to catch, and it would pass all three tests
    // above while doing it.
    let p = project("reproducible", 0, false);
    let (ok, output) = p.round();

    for label in LABELS {
        assert!(
            output.contains(&format!("CAUGHT        {label}")),
            "the round did not judge {label:?}:\n{output}"
        );
    }
    assert!(
        ok,
        "a round over a reproducible build did not end cleanly:\n{output}"
    );
    assert!(
        !output.contains("retaken") && !output.contains("retak"),
        "a build that reproduces made the harness retake its baseline anyway:\n{output}"
    );
    assert!(
        !output.contains("not judged:"),
        "a round that ended cleanly still reported a case it never measured:\n{output}"
    );
}

/// The control, driven by the harness as it was before this repair.
///
/// Kept because the repair is a behaviour and not a line: without this the
/// tests above pass on a harness that never had the defect, and nothing says
/// the fixture reproduces the thing that cost six cases. The pre-repair script
/// is read out of git rather than kept as a copy, so it cannot drift into
/// agreeing with the current one.
///
/// ⚠ The revision is PINNED, and the first draft of this test was not — it
/// asked git for the newest commit that had touched `scripts/mutate`, which the
/// moment the repair landed was the repair itself. It then compared the fix
/// against the fix, found no defect, printed a line to stderr and passed. That
/// is this repository's recorded failure shape for a control built on an
/// absence: the sweep goes empty and the emptiness reads as green. So every way
/// out of this test is now a FAILURE with a sentence. If a later change to the
/// libraries this pinned script sources makes it undrivable, that is a decision
/// for a person — retire the control deliberately — and not something the test
/// may take silently.
const DEFECT_REVISION: &str = "79f1bf284d2010af4b727ae79e3c6f941621dce6";

#[test]
fn the_harness_this_replaces_ends_the_casefile_at_its_first_case() {
    let older = Command::new("git")
        .arg("show")
        .arg(format!("{DEFECT_REVISION}:scripts/mutate"))
        .current_dir(repo_root())
        .output()
        .expect("git show scripts/mutate");
    assert!(
        older.status.success(),
        "the pinned pre-repair harness {DEFECT_REVISION} could not be read, so this \
         control measured nothing:\n{}",
        String::from_utf8_lossy(&older.stderr)
    );

    let dir = tempdir().expect("temp dir");
    let harness = dir.path().join("mutate");
    fs::write(&harness, &older.stdout).expect("write the earlier harness");
    let mut perms = fs::metadata(&harness).expect("stat").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&harness, perms).expect("make it executable");

    let p = project("control", 0, true);
    let (ok, output) = p.round_with(&harness);
    // The half that gives the other three their meaning: this fixture really
    // does reproduce the defect, in the pre-repair harness's own words.
    assert!(
        output.contains("restore did not reproduce the baseline binaries"),
        "the fixture no longer reproduces the defect this repair is about, so \
         nothing here measures it:\n{output}"
    );
    assert!(
        !ok,
        "the earlier harness ended a round it could not finish:\n{output}"
    );
    assert!(
        output.contains(&format!("CAUGHT        {}", LABELS[0])),
        "the control did not get as far as its first verdict:\n{output}"
    );
    assert!(
        !output.contains(&format!("CAUGHT        {}", LABELS[1])),
        "the control was supposed to stop before its second case:\n{output}"
    );
}

#[test]
fn a_baseline_that_cannot_be_retaken_stops_the_round_and_names_what_it_lost() {
    // Five is the first case's restore build, counted rather than guessed: the
    // fixture builds once, `mutation_ctest_resolve` builds before it resolves,
    // the baseline builds, and then each case builds twice — once mutated and
    // once restored. So the suite goes red for a reason restoring files cannot
    // undo, at the exact moment the round asks whether it still has a
    // baseline. Continuing there would measure the two cases below against a
    // tree that really has moved.
    let p = project("cannot-retake", 5, true);
    let (ok, output) = p.round();

    // The margin the count above leaves. A marker that landed earlier would
    // stop the round with a different sentence, and asserting on that sentence
    // without this would read as the branch under test having fired.
    assert!(
        output.contains("baseline: 1 tests, 0 failing"),
        "the round never got a green baseline, so it stopped somewhere else:\n{output}"
    );
    assert!(
        !ok,
        "a round that could not retake its baseline passed:\n{output}"
    );
    assert!(
        output.contains("the baseline could not be retaken"),
        "the round stopped for some other reason:\n{output}"
    );
    assert!(
        output.contains(&format!("CAUGHT        {}", LABELS[0])),
        "the case that WAS judged is missing from the output:\n{output}"
    );
    for label in &LABELS[1..] {
        assert!(
            output.contains(&format!("not judged: {label}")),
            "the round did not name {label:?} as one it never measured:\n{output}"
        );
    }

    // And the record holds them, which is the half a stopped round used to
    // lose entirely: the verdict it did earn, and the names of the cases it
    // did not reach.
    let record = p.record();
    assert!(
        record.contains("\"rc\": 2"),
        "the record reads like a round that reached its end:\n{record}"
    );
    assert!(
        record.contains("\"verdict\": \"CAUGHT\""),
        "the verdict the round did earn is not in its record:\n{record}"
    );
    for label in &LABELS[1..] {
        assert!(
            record.contains(&format!("\"label\": \"{label}\"")),
            "the record does not name {label:?}:\n{record}"
        );
    }
    assert_eq!(
        record.matches("\"verdict\": \"UNJUDGED\"").count(),
        2,
        "the record should carry one unjudged case per case never reached:\n{record}"
    );
}
