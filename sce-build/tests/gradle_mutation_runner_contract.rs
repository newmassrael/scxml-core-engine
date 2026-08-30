// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The two guards that make a Gradle mutation round's green mean anything.
//!
//! A mutation round reads a verdict off a test run, and Gradle is the runner
//! in this repository that will happily hand one back without running
//! anything. Two of its ordinary features do it: up-to-date checking skips a
//! task whose declared inputs have not moved, and the build cache — this
//! repository sets `org.gradle.caching=true` — RESTORES a task's outputs,
//! JUnit XML included, from a previous run. Either one turns a mutation round
//! into a reading of an earlier build.
//!
//! Measured 2026-08-30, with both escapes removed from the init script and
//! `:sce-kotlin-tests:test --tests com.sce.w3c.GateEnginePairsTest` invoked
//! twice in a row: the second invocation answered `FROM-CACHE`, wrote **no**
//! classpath listing — and still left **one** JUnit XML behind, green, with
//! two passing tests in it. A harness reading only the report would have
//! called that a run.
//!
//! So there are two guards, in two files, and they are complementary rather
//! than redundant:
//!
//! * `scripts/lib/mutation-gradle-init.gradle` closes both escapes at the
//!   task, so the situation does not arise; and
//! * `scripts/mutate`'s Gradle runner refuses a report that no test task
//!   produced, so if the first guard is ever lost the round says so instead
//!   of reading the stale report.
//!
//! The second was measured against the first the same day: with the escapes
//! removed, a full round over `kotlin_engine_pairs_jvm.cases` exited 2 at the
//! baseline — "no Test task executed" — rather than reporting verdicts drawn
//! from a cache. Without it the same round would have compared every case
//! against a baseline that was itself restored.
//!
//! These are text predicates over two shell files, which is what this
//! repository can hold them with on every push; what they are NOT is a
//! substitute for the behavioural measurement above, and a third way of
//! defeating the same property — a Gradle release that stops running
//! `doFirst` on an executed task, say — would pass all three. That residue is
//! named here rather than left to be discovered.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

const INIT_SCRIPT: &str = "scripts/lib/mutation-gradle-init.gradle";
const HARNESS: &str = "scripts/mutate";

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// The same text with its commented-out lines gone.
///
/// Every predicate here reads source for a line it expects to find, and a
/// guard COMMENTED OUT is still that line as far as a substring search is
/// concerned. Measured 2026-08-30, the first round over this file's casefile:
/// prefixing `outputs.cacheIf { false }` with `//` left both this test and the
/// guard's own words in the file, and the case came back SURVIVED — the
/// predicate was reading the disabled copy.
///
/// Line-leading markers only, and both dialects, because that is what a
/// disabled line looks like and it cannot mistake a `#` or a `//` inside a
/// string for one. A guard given a trailing comment is still a live guard, so
/// there is nothing to strip there.
fn code_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with('#')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The body of one shell function, from its opening line to the closing brace
/// in the first column.
///
/// Sliced rather than searched whole, because "the file mentions this string"
/// and "this function does this" are different claims and only the second is
/// worth asserting: the file's own header explains both escapes at length, so
/// a check over the whole text would pass on the prose alone.
fn shell_function(source: &str, name: &str) -> String {
    let source = code_lines(source);
    let source = source.as_str();
    let opening = format!("{name}() {{");
    let start = source
        .find(&opening)
        .unwrap_or_else(|| panic!("{HARNESS} declares no function {name}"));
    let rest = &source[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("{name} in {HARNESS} has no closing brace in column one"));
    rest[..end].to_string()
}

/// Both escapes a Gradle test task has are closed where its tasks are
/// configured.
#[test]
fn the_init_script_closes_both_escapes_a_cached_test_task_has() {
    let script = code_lines(&read(INIT_SCRIPT));

    // Not the file, the configuration block: `configureEach` is where a
    // setting reaches every test task of every project, and a call sitting
    // anywhere else would be describing a build this harness does not run.
    let configure_at = script
        .find("tasks.withType(Test).configureEach")
        .expect("the init script configures every Test task of every project");
    let block = &script[configure_at..];

    for escape in [
        "outputs.upToDateWhen { false }",
        "outputs.cacheIf { false }",
    ] {
        assert!(
            block.contains(escape),
            "⚠ {INIT_SCRIPT} no longer sets `{escape}` on the Test tasks it \
             configures. Measured 2026-08-30 with both removed: Gradle \
             answered the second identical invocation FROM-CACHE and restored \
             a green JUnit XML for a run that never happened. Up-to-date \
             checking and the build cache are separate escapes — closing one \
             leaves the other open."
        );
    }
}

/// The compile step of a round runs no test, so its exit status is a compile
/// verdict.
///
/// Without this the two questions a round has to keep apart — did the mutated
/// tree still build, and did the suite turn red — arrive as one non-zero exit,
/// and every caught mutation would be indistinguishable from one the compiler
/// refused.
#[test]
fn the_compile_step_of_a_round_runs_no_test() {
    let script = code_lines(&read(INIT_SCRIPT));
    let configure_at = script
        .find("tasks.withType(Test).configureEach")
        .expect("the init script configures every Test task of every project");
    let block = &script[configure_at..];

    assert!(
        block.contains("if (compileOnly)") && block.contains("testTask.enabled = false"),
        "⚠ {INIT_SCRIPT} no longer disables the Test tasks under \
         `sce.mutation.compileOnly`. The Gradle runner's build step is what \
         separates `the tree stopped compiling` from `the suite turned red`, \
         and it separates them by running a Gradle invocation that CANNOT \
         run a test."
    );
}

/// The runner refuses a report that no test task produced.
///
/// The second guard, and the one that survives the first being lost. A task
/// restored from the build cache has its report restored with it; what is not
/// restored is the task's own actions, so the classpath listing the init
/// script writes from `doFirst` is absent exactly when the report is stale.
#[test]
fn the_runner_refuses_a_report_no_test_task_produced() {
    let harness = read(HARNESS);
    let body = shell_function(&harness, "mutation_gradle_run");

    assert!(
        body.contains("rm -rf \"$MUTATION_GRADLE_REPORT_DIR\" \"$MUTATION_GRADLE_CLASSPATH_DIR\""),
        "⚠ mutation_gradle_run no longer clears the classpath directory before \
         invoking Gradle. The listing left by a PREVIOUS invocation would then \
         answer for this one, and the guard below would pass on evidence from \
         the wrong run."
    );

    // The whole condition, negation included. Asserting the `compgen` call
    // alone would pass on a guard whose sense had been inverted — the line
    // still present, the refusal now firing on the runs that DID execute —
    // which is the "checks a constant is NAMED rather than USED" defect this
    // repository has already paid for once.
    assert!(
        body.contains("if ! compgen -G \"$MUTATION_GRADLE_CLASSPATH_DIR/*.txt\" >/dev/null; then"),
        "⚠ mutation_gradle_run no longer asks whether any test task actually \
         executed, or no longer asks it the right way round. Measured \
         2026-08-30: with the init script's escapes removed a full round \
         stopped at the baseline with `no Test task executed` rather than \
         reading a restored report — that refusal is this line."
    );

    assert!(
        body.contains("> \"$MUTATION_UNREADABLE\""),
        "⚠ mutation_gradle_run no longer reports the absent run through \
         MUTATION_UNREADABLE. `the suite ran nothing` and `the suite was never \
         asked` are different facts, and only the first is about the mutation; \
         folding the second into a count of zero would blame the case for the \
         runner."
    );
}
