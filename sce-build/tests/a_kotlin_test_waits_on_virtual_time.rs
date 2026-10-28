// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Kotlin test waits for a machine on a clock it owns, not on the wall.
//
// `ATargetExpressionIsRoutedAsItsLiteralIs` polled for 3000ms of real time and
// finished in 40ms alone. Run with the rest of the module in parallel it missed
// the deadline, and it failed there and nowhere else: a verdict that depends on
// how soon the thread is scheduled says nothing about the clause under test.
// Thirty tests had the same loop (`currentTimeMillis` + `Thread.sleep`), and
// lengthening each deadline would only have moved the mark, because the
// document's own timers are armed in the same real seconds.
//
// The loop is `driveOnVirtualTime` in `VirtualDrive.kt`, and this test holds the
// suite to it: a file that reads the wall clock or sleeps is refused unless it
// is named below with the reason a virtual clock cannot stand in for it. Each
// entry must still read the wall, so a file that was converted is removed from
// the list rather than left as a licence for the next one.
//
// Two traps avoided by construction:
//
// COMMENTS ARE STRIPPED FIRST. The migrated tests explain what they replaced, and
// `VirtualDrive.kt` names the very calls it removes; a scan of raw text would
// refuse the files that say why they are clean. Strings are not stripped, so a
// `"//"` in a literal would cut its line short: that can only hide a later hit
// on the same line, and no test here puts one there.
//
// A POPULATION THAT GOES EMPTY REPORTS NO VIOLATIONS. The floor below reads the
// tree's own size, so a moved directory fails here rather than passing quietly.

use std::fs;
use std::path::{Path, PathBuf};

/// What reads the wall, and why a virtual clock cannot do it instead.
const REAL_TIME: [(&str, &str); 3] = [
    (
        "com/sce/integration/ADelayedHttpSendIsPostedWhenDueTest.kt",
        "its second test runs the engine's coroutine on Dispatchers.Default and measures real elapsed time",
    ),
    (
        "com/sce/integration/LateTickHonoursCancelTest.kt",
        "two tests are the wall-clock probes of timeUntilNextScheduledMs, compared against their virtual twins",
    ),
    (
        "com/sce/w3c/W3CHttpTestBase.kt",
        "a mock HTTP server's own thread injects the response events, which no virtual clock can make arrive",
    ),
];

const WALL: [&str; 3] = ["currentTimeMillis", "Thread.sleep", "nanoTime"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .join("backends/kotlin/tests/src/test/kotlin")
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("the Kotlin test tree is readable") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "kt") {
            out.push(path);
        }
    }
}

/// The code of a Kotlin file with its comments removed.
fn code_of(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("/*") {
        stripped.push_str(&rest[..open]);
        rest = match rest[open + 2..].find("*/") {
            Some(close) => &rest[open + 2 + close + 2..],
            None => "",
        };
    }
    stripped.push_str(rest);
    stripped
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn reads_the_wall(code: &str) -> bool {
    WALL.iter().any(|call| code.contains(call))
}

fn files_reading_the_wall() -> (usize, Vec<String>) {
    let base = root();
    let mut files = Vec::new();
    sources(&base, &mut files);
    let mut reading = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).expect("a Kotlin test source is text");
        if reads_the_wall(&code_of(&text)) {
            let relative = file.strip_prefix(&base).expect("under the test tree");
            reading.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    reading.sort();
    (files.len(), reading)
}

/// The files that read the wall without a reason named for them.
fn unnamed<'a>(reading: &'a [String], allowed: &[&str]) -> Vec<&'a String> {
    reading
        .iter()
        .filter(|file| !allowed.contains(&file.as_str()))
        .collect()
}

/// The named files that no longer read the wall.
fn stale<'a>(allowed: &[&'a str], reading: &[String]) -> Vec<&'a str> {
    allowed
        .iter()
        .copied()
        .filter(|file| !reading.iter().any(|r| r == file))
        .collect()
}

fn allowed() -> Vec<&'static str> {
    REAL_TIME.iter().map(|(file, _)| *file).collect()
}

#[test]
fn only_a_named_test_reads_the_wall_clock() {
    let (population, reading) = files_reading_the_wall();
    assert!(
        population >= 100,
        "the Kotlin test tree holds {population} sources; the scan read a tree that is not the suite"
    );
    let unnamed = unnamed(&reading, &allowed());
    assert!(
        unnamed.is_empty(),
        "{} Kotlin test(s) wait on the wall clock (currentTimeMillis, Thread.sleep or nanoTime). \
         Start the machine with `initializeOnVirtualTime()` and wait with `driveOnVirtualTime(...)` \
         from VirtualDrive.kt, so the verdict does not depend on how loaded the host is. If the wait \
         is real I/O, name the file in REAL_TIME with the reason:\n  {}",
        unnamed.len(),
        unnamed.iter().map(|f| f.as_str()).collect::<Vec<_>>().join("\n  ")
    );
}

#[test]
fn a_named_exception_still_reads_the_wall_clock() {
    let (_, reading) = files_reading_the_wall();
    let stale = stale(&allowed(), &reading);
    assert!(
        stale.is_empty(),
        "REAL_TIME names file(s) that no longer read the wall clock; remove them so the list stays \
         the set of waits virtual time cannot do:\n  {}",
        stale.join("\n  ")
    );
}

#[test]
fn a_comment_that_names_the_call_is_not_a_use_of_it() {
    let text = "// Thread.sleep(10) was here\nval a = 1 /* currentTimeMillis */ + 2\n/* nanoTime\n nanoTime */\n";
    assert!(!reads_the_wall(&code_of(text)));
}

#[test]
fn each_way_of_reading_the_wall_is_seen() {
    for call in [
        "System.currentTimeMillis()",
        "Thread.sleep(10)",
        "System.nanoTime()",
    ] {
        assert!(
            reads_the_wall(&code_of(&format!("val t = {call}\n"))),
            "{call}"
        );
    }
}

#[test]
fn the_two_lists_are_told_apart_by_name() {
    let reading = vec!["a.kt".to_string(), "b.kt".to_string()];
    assert_eq!(unnamed(&reading, &["a.kt"]), vec![&reading[1]]);
    assert_eq!(stale(&["a.kt", "c.kt"], &reading), vec!["c.kt"]);
}
