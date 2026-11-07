// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The heap a Kotlin lane row used is read from the JVM's own GC log, and this
//! suite holds that reading to what it claims.
//!
//! `scripts/gates/w3c-kotlin.sh` starts the Gradle daemon and the Kotlin
//! compile daemon with `-Xlog:gc`, notes where each log ended before a row,
//! and asks `scripts/gates/jvm_heap_report.py` about what the row added. It
//! exists because the `lua:lua` row ran out of heap on every run on main from
//! 2026-11-05 while no build host reproduced it (2026-11-07): the number a
//! budget must be sized from could be read only in the failing run.
//!
//! A report nobody can check is a second unread number, so its rules are cases
//! here, over logs built from the line format the JDK writes. The format was
//! taken from a real run of OpenJDK 21 with the gate's exact flag
//! (`-Xlog:gc:file=…:uptime,level,tags:filecount=0`, measured 2026-11-07):
//!
//! ```text
//! [0.002s][info][gc] Using Parallel
//! [0.456s][info][gc] GC(0) Pause Young (Allocation Failure) 64M->12M(245M) 4.102ms
//! [0.528s][info][gc] GC(7) Pause Full (System.gc()) 103M->103M(245M) 15.522ms
//! ```
//!
//! ⚠ A refusal is exit code 3, asserted as that number: a reader that died on
//! its way to a decision also exits non-zero, and the two must not read alike.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A decision this reader made, and the answer is no.
const REFUSED: i32 = 3;

fn program() -> PathBuf {
    let program = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .join("scripts/gates/jvm_heap_report.py");
    assert!(
        program.is_file(),
        "no heap reader at {} — `scripts/gates/w3c-kotlin.sh` asks it about every \
         row, so its absence is a lane whose heap nobody reads",
        program.display()
    );
    program
}

struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn report(log: &Path, from_byte: u64, jvmargs: &str, extra: &[&str], github: bool) -> Outcome {
    let mut command = Command::new("python3");
    command
        .arg(program())
        .arg("report")
        .arg("--log")
        .arg(log)
        .arg("--from-byte")
        .arg(from_byte.to_string())
        // `=`-joined, as the gate passes it: JVM arguments open with `-`, and a
        // separate value with no space in it (`-XX:+UseParallelGC`) is read by
        // argparse as an option rather than as the value.
        .arg(format!("--jvmargs={jvmargs}"))
        .arg("--label")
        .arg("the row")
        .args(extra);
    if github {
        command.env("GITHUB_ACTIONS", "true");
    } else {
        command.env_remove("GITHUB_ACTIONS");
    }
    let out = command.output().expect("python3 runs the heap reader");
    Outcome {
        code: out
            .status
            .code()
            .expect("the program exited rather than signalled"),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// One collection line as the JDK writes it on `gc` with the gate's decorations.
fn collection(uptime: f64, id: u32, full: bool, before_m: u32, after_m: u32, ms: f64) -> String {
    let what = if full {
        "Pause Full (Ergonomics)"
    } else {
        "Pause Young (Allocation Failure)"
    };
    format!("[{uptime:.3}s][info][gc] GC({id}) {what} {before_m}M->{after_m}M(1964M) {ms:.3}ms\n")
}

fn log_of(lines: &[String]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("gc.log");
    let mut text = String::from("[0.002s][info][gc] Using Parallel\n");
    for line in lines {
        text.push_str(line);
    }
    fs::write(&path, text).expect("the log is writable");
    (dir, path)
}

const BUDGET: &str = "-Xmx2g -XX:+UseParallelGC";

#[test]
fn a_row_reports_its_peak_its_live_set_and_its_budget() {
    let (_dir, log) = log_of(&[
        collection(1.0, 0, false, 900, 300, 10.0),
        collection(2.0, 1, false, 1200, 600, 20.0),
        collection(3.0, 2, true, 1000, 700, 500.0),
    ]);
    let out = report(&log, 0, BUDGET, &[], false);

    assert_eq!(
        out.code, 0,
        "a readable row was not reported\nstderr: {}",
        out.stderr
    );
    for expected in [
        "3 collection(s)",
        "peak 1200 MB",
        "live 700 MB after the largest of 1 full collection(s)",
        "budget -Xmx 2048 MB",
        "(34% of it live)",
    ] {
        assert!(
            out.stdout.contains(expected),
            "the report does not say `{expected}`\nstdout: {}",
            out.stdout
        );
    }
    assert!(
        !out.stderr.contains("WARNING"),
        "a live set of a third of the budget was warned about\nstderr: {}",
        out.stderr
    );
}

/// The share of the row spent in pauses is what the JVM's "GC overhead limit
/// exceeded" is about, so it is reported and read here.
#[test]
fn the_share_of_the_row_spent_in_pauses_is_reported() {
    let (_dir, log) = log_of(&[
        collection(10.0, 0, true, 1900, 1850, 1500.0),
        collection(12.0, 1, true, 1900, 1860, 1500.0),
    ]);
    let out = report(&log, 0, BUDGET, &[], false);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    // A line is written when its pause ends, so the span opens 1.5s before the
    // first line: 3.0s of pauses over the 3.5s from 8.5s to 12.0s.
    assert!(
        out.stdout.contains("85.7% of 3.5s in pauses"),
        "the pause share was not reported as pause time over the span from the first \
         pause's start to the last pause's end\nstdout: {}",
        out.stdout
    );
}

#[test]
fn a_live_set_close_to_the_budget_is_a_warning_and_not_a_refusal() {
    let (_dir, log) = log_of(&[collection(5.0, 0, true, 1900, 1700, 900.0)]);

    let out = report(&log, 0, BUDGET, &[], false);
    assert_eq!(
        out.code, 0,
        "a live set near the budget refused the row (exit {}). The budget is a choice \
         made from these numbers, so the reader warns and leaves the row's verdict alone\n\
         stderr: {}",
        out.code, out.stderr
    );
    assert!(
        out.stderr.contains(
            "WARNING the row: the live set is 1700 MB, at or above 80% of the 2048 MB budget"
        ),
        "no warning for a live set at 83% of the budget\nstderr: {}",
        out.stderr
    );
    assert!(
        !out.stdout.contains("::warning"),
        "an annotation was written outside GitHub Actions\nstdout: {}",
        out.stdout
    );

    let on_github = report(&log, 0, BUDGET, &[], true);
    assert!(
        on_github
            .stdout
            .contains("::warning title=JVM heap close to its budget::"),
        "under GitHub Actions the warning is not an annotation, so the run page would \
         not show a lane closing on its budget\nstdout: {}",
        on_github.stdout
    );
}

/// The gate notes where a log ended before a row and asks only about what the
/// row added: a daemon reused across rows writes one log for all of them.
#[test]
fn only_what_the_row_added_is_read() {
    let earlier = collection(1.0, 0, true, 1900, 1800, 900.0);
    let (_dir, log) = log_of(&[earlier.clone(), collection(20.0, 1, true, 500, 200, 50.0)]);
    let from = fs::read_to_string(&log)
        .expect("the log is readable")
        .find("[20.000s]")
        .expect("the second row's line is in the log") as u64;

    let out = report(&log, from, BUDGET, &[], false);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert!(
        out.stdout.contains("1 collection(s)") && out.stdout.contains("live 200 MB"),
        "the report read the earlier row's collection too\nstdout: {}",
        out.stdout
    );
    assert!(
        !out.stderr.contains("WARNING"),
        "the earlier row's live set was charged to this one\nstderr: {}",
        out.stderr
    );
}

#[test]
fn a_row_with_no_full_collection_reports_its_live_set_as_a_bound_only() {
    let (_dir, log) = log_of(&[collection(1.0, 0, false, 1900, 1750, 10.0)]);
    let out = report(&log, 0, BUDGET, &[], false);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert!(
        out.stdout
            .contains("no full collection, so the live set is at most 1750 MB"),
        "a young collection's after-heap was reported as the live set; it still holds \
         the old generation's garbage\nstdout: {}",
        out.stdout
    );
    assert!(
        !out.stderr.contains("WARNING"),
        "a bound was warned about as if it were the live set\nstderr: {}",
        out.stderr
    );
}

#[test]
fn a_log_that_must_exist_and_does_not_is_refused() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let missing = dir.path().join("never-written.log");

    let refused = report(&missing, 0, BUDGET, &["--must-exist"], false);
    assert_eq!(
        refused.code, REFUSED,
        "a GC log the JVM was told to write and did not was not refused (exit {}). A \
         report of no collections from a log nobody wrote would read as a row that used \
         no heap\nstdout: {}\nstderr: {}",
        refused.code, refused.stdout, refused.stderr
    );
    assert!(
        refused.stderr.contains("its heap was not observed"),
        "the refusal does not say the row went unobserved\nstderr: {}",
        refused.stderr
    );

    // The control: a JVM that did not start during the row (the Kotlin daemon
    // when nothing compiled) is said to have not started, not refused.
    let absent = report(&missing, 0, BUDGET, &[], false);
    assert_eq!(absent.code, 0, "stderr: {}", absent.stderr);
    assert!(
        absent.stdout.contains("did not start during the row"),
        "a daemon that did not start was not said to have not started\nstdout: {}",
        absent.stdout
    );
}

#[test]
fn arguments_with_no_heap_limit_are_refused() {
    let (_dir, log) = log_of(&[collection(1.0, 0, true, 100, 50, 5.0)]);
    let out = report(&log, 0, "-XX:+UseParallelGC", &[], false);
    assert_eq!(
        out.code, REFUSED,
        "a row was read against no budget (exit {})\nstdout: {}\nstderr: {}",
        out.code, out.stdout, out.stderr
    );
    assert!(
        out.stderr.contains("carry no -Xmx"),
        "the refusal does not name the missing -Xmx\nstderr: {}",
        out.stderr
    );
}
