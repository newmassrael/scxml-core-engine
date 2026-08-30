// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A red baseline says WHY, not only which.
//
// `scripts/mutate` refuses to start a round whose baseline is not green. Until
// 2026-08-30 that refusal carried a count and a list of names, and the runner's
// own account of those failures — the panic, the traceback, the expectation
// block — was parsed for names and dropped one pipe later. Measured that day on
// a probe casefile: the refusal read
//
//     baseline is not green (1 failing) — fix that before mutating,
//                   test_oracle.py::test_value_is_two
//
// and nothing else, while pytest had printed the assertion, both values it
// compared and the line it stood on.
//
// The cost is not hypothetical. A leaked `SCE_MUTATION_SHARD` held
// `mutation_rounds_selection.cases` red for six days, and the CI job's log
// carried zero occurrences of `panicked at` or `assertion` — because this is
// where the runner's account of it went. The build-failure branch has had its
// counterpart since the beginning (`mutation_build_diagnostics`); the
// run-failure branch had none.
//
// This file holds three different kinds of evidence, because each covers what
// the others cannot:
//
//   1. THE PARSERS, against captured runner output. Five formats, five parsers,
//      all owned by somebody else and therefore the part most likely to drift.
//   2. THE PLUMBING, out of the script's own text. A parser that works and is
//      never called restores the exact silence being repaired, and the two
//      heaviest runners (ctest, gradle) cost minutes to exercise for real — so
//      the population is DERIVED from `mutation_declare_runner` calls rather
//      than listed here, and a sixth runner added tomorrow is measured on the
//      day it lands.
//   3. THE BEHAVIOUR, by running a real round whose baseline is red. One
//      runner, end to end, including which stream the account arrives on.
//
// Two rules this repository paid for shape the scanning in (2). COMMENTS ARE
// STRIPPED FIRST, because a scanner satisfied by the prose around the code has
// happened here more than once and this script explains itself at length. AND
// THERE IS A FLOOR: a source scan whose population goes empty reports zero
// violations, which reads exactly like a clean tree.

use std::collections::BTreeSet;
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

// ── 1. The parsers ─────────────────────────────────────────────────

/// Feed captured runner output to one of the parsers in
/// `scripts/lib/mutation_failures.sh` and return what it wrote.
fn parser_output(function: &str, captured: &str) -> String {
    let dir = tempdir().expect("temp dir");
    let sample = dir.path().join("captured.txt");
    fs::write(&sample, captured).expect("write the sample");

    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source scripts/lib/mutation_failures.sh; {function} < {}",
            sample.display()
        ))
        .current_dir(repo_root())
        .output()
        .expect("run the parser");
    assert!(
        out.status.success(),
        "{function} exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn gtest_detail_is_the_bracket_around_the_failure_and_not_the_summary() {
    // gtest brackets a test with `[ RUN      ]` and closes it with a verdict;
    // the diagnosis is what it prints in between. The end-of-run summary
    // repeats `[  FAILED  ]` with no bracket open in front of it, and reading
    // that as a failure's detail would attach a test's name to nothing.
    let detail = parser_output(
        "mutation_detail_from_gtest",
        "2: [ RUN      ] Suite.Green\n\
         2: [       OK ] Suite.Green (0 ms)\n\
         2: [ RUN      ] Suite.Red\n\
         2: /w/x.cc:12: Failure\n\
         2: Expected equality of these values:\n\
         2:     Which is: 1\n\
         2: [  FAILED  ] Suite.Red (1 ms)\n\
         2: [  FAILED  ] 1 test, listed below:\n\
         2: [  FAILED  ] Suite.Red\n",
    );
    assert!(
        detail.contains("/w/x.cc:12: Failure") && detail.contains("Which is: 1"),
        "the gtest detail parser must carry the expectation block:\n{detail}"
    );
    assert!(
        !detail.contains("Suite.Green"),
        "a passing test's bracket is not a failure's detail:\n{detail}"
    );
    assert!(
        !detail.contains("1 test, listed below"),
        "the end-of-run summary opens no bracket and must not be read as one:\n{detail}"
    );
}

#[test]
fn cargo_detail_is_the_captured_block_and_stops_at_the_index() {
    // libtest replays a failing test's output under `---- <name> stdout ----`
    // and closes the section with the `failures:` index the name parser reads.
    // That index is the terminator because a panic's own text can hold
    // anything, a line of dashes included.
    let detail = parser_output(
        "mutation_detail_from_cargo",
        "running 2 tests\n\
         test a_pass ... ok\n\
         test inner::a_fail ... FAILED\n\n\
         failures:\n\n\
         ---- inner::a_fail stdout ----\n\
         thread 'inner::a_fail' panicked at sce-build/tests/x.rs:12:5:\n\
         assertion `left == right` failed\n\
         \x20 left: 1\n\
         \x20right: 2\n\n\n\
         failures:\n    inner::a_fail\n\n\
         test result: FAILED. 1 passed; 1 failed; 0 ignored\n",
    );
    assert!(
        detail.contains("panicked at sce-build/tests/x.rs:12:5")
            && detail.contains("left: 1")
            && detail.contains("right: 2"),
        "the cargo detail parser must carry the panic and the values:\n{detail}"
    );
    assert!(
        !detail.contains("test result:"),
        "the block ends at the `failures:` index, so the run summary is outside it:\n{detail}"
    );
}

#[test]
fn go_detail_carries_the_log_line_and_the_failing_subtest() {
    // A table-driven Go test names the broken row in an INDENTED verdict under
    // the parent's. The name parser deliberately counts the parent only, so
    // the row name exists nowhere else in what the harness keeps.
    let detail = parser_output(
        "mutation_detail_from_go",
        "=== RUN   TestGreen\n\
         --- PASS: TestGreen (0.00s)\n\
         === RUN   TestRed\n\
         === RUN   TestRed/row_two\n\
         \x20   thing_test.go:41: the raiser answered 1, want 2\n\
         --- FAIL: TestRed (0.00s)\n\
         \x20   --- FAIL: TestRed/row_two (0.00s)\n\
         FAIL\n\
         exit status 1\n",
    );
    assert!(
        detail.contains("thing_test.go:41: the raiser answered 1, want 2"),
        "the go detail parser must carry the logged line:\n{detail}"
    );
    assert!(
        detail.contains("--- FAIL: TestRed/row_two"),
        "the failing subtest's own verdict must survive:\n{detail}"
    );
    assert!(
        !detail.contains("TestGreen"),
        "a passing test contributes nothing:\n{detail}"
    );
}

#[test]
fn pytest_detail_is_the_failures_section_and_stops_at_the_summary() {
    // The `FAILURES` banner opens the section holding the source line, the `E`
    // line and the traceback; `short test summary info` closes it, and what
    // follows is the one-line list the name parser already reads.
    let detail = parser_output(
        "mutation_detail_from_pytest",
        "F                                                            [100%]\n\
         =================================== FAILURES ===================================\n\
         ____________________________ test_value_is_two _____________________________\n\n\
         \x20   def test_value_is_two():\n\
         >       assert subject.value() == 2\n\
         E       assert 1 == 2\n\n\
         test_oracle.py:5: AssertionError\n\
         =========================== short test summary info ============================\n\
         FAILED test_oracle.py::test_value_is_two - assert 1 == 2\n\
         1 failed in 0.01s\n",
    );
    assert!(
        detail.contains("E       assert 1 == 2") && detail.contains("test_oracle.py:5"),
        "the pytest detail parser must carry the assertion and its line:\n{detail}"
    );
    assert!(
        !detail.contains("1 failed in"),
        "the section ends at the summary banner:\n{detail}"
    );
}

#[test]
fn a_collection_error_is_detail_too() {
    // A mutation upstream of an import fails at collection, which pytest
    // reports under an `ERRORS` banner rather than `FAILURES`. Reading only
    // the one banner would leave the shape a Python round most needs to
    // explain with no account at all.
    let detail = parser_output(
        "mutation_detail_from_pytest",
        "==================================== ERRORS ====================================\n\
         _______________________ ERROR collecting test_oracle.py ________________________\n\
         ImportError while importing test module 'test_oracle.py'.\n\
         E   ModuleNotFoundError: No module named 'subject'\n\
         =========================== short test summary info ============================\n\
         ERROR test_oracle.py\n",
    );
    assert!(
        detail.contains("ModuleNotFoundError: No module named 'subject'"),
        "an ERRORS banner is an account of a mutation, and must be kept:\n{detail}"
    );
}

#[test]
fn a_green_run_produces_no_detail() {
    // The direction that matters for a round that is about to proceed: a
    // parser that invented detail on a passing run would put an account of a
    // failure under a baseline that had none.
    for (parser, captured) in [
        (
            "mutation_detail_from_gtest",
            "1: [ RUN      ] Suite.Green\n1: [       OK ] Suite.Green (0 ms)\n\
             1: [  PASSED  ] 1 test.\n",
        ),
        (
            "mutation_detail_from_cargo",
            "running 1 test\ntest a_thing ... ok\n\n\
             test result: ok. 1 passed; 0 failed; 0 ignored\n",
        ),
        (
            "mutation_detail_from_go",
            "=== RUN   TestGreen\n--- PASS: TestGreen (0.00s)\nPASS\n",
        ),
        (
            "mutation_detail_from_pytest",
            ".                                                            [100%]\n\
             1 passed in 0.01s\n",
        ),
    ] {
        let detail = parser_output(parser, captured);
        assert!(
            detail.trim().is_empty(),
            "{parser} invented detail for a green run:\n{detail}"
        );
    }
}

#[test]
fn an_empty_account_says_so_rather_than_printing_a_silence() {
    // The failure mode this whole repair is about, one level down. A detail
    // parser that drifts off its runner's format produces nothing, and nothing
    // reads as "no trouble" rather than as "no reading" — which is how the
    // silence would come back wearing the repair's own clothes.
    let spoken = parser_output("mutation_baseline_detail", "");
    assert!(
        spoken.contains("printed nothing this harness could read"),
        "an empty account must be reported, not skipped:\n{spoken}"
    );
    assert!(
        spoken.contains("may have drifted"),
        "and it must name the reason a reader can act on:\n{spoken}"
    );
}

#[test]
fn a_long_account_is_capped_and_says_how_many_lines_it_hid() {
    // A baseline broken wholesale can print thousands of lines, and the
    // refusal above them is what the reader must not lose off the top of the
    // screen. Truncating silently would be the same defect one size down.
    let many: String = (0..400).map(|i| format!("line {i}\n")).collect();
    let out = parser_output("mutation_baseline_detail", &many);
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.len() < 400,
        "a 400-line account must not be printed whole: {} lines",
        lines.len()
    );
    assert!(
        lines.last().is_some_and(|l| l.contains("more line(s)")),
        "the cap must say how many lines it hid:\n{out}"
    );
    assert!(
        out.contains("line 0"),
        "and it must keep the head, where the first failure is:\n{out}"
    );
}

/// Run `mutation_gradle_report` — cut out of `scripts/mutate` itself, so this
/// exercises the shipped text rather than a copy — over a JUnit tree.
fn gradle_report_detail(junit_xml: &str) -> String {
    let dir = tempdir().expect("temp dir");
    let reports = dir.path().join("reports");
    fs::create_dir_all(&reports).expect("create the report directory");
    fs::write(reports.join("TEST-a.xml"), junit_xml).expect("write the report");
    let failing = dir.path().join("failing.txt");
    let detail = dir.path().join("detail.txt");

    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -e\n\
             eval \"$(sed -n '/^mutation_gradle_report() {{/,/^}}/p' scripts/mutate)\"\n\
             MUTATION_GRADLE_REPORT_DIR={reports} MUTATION_FAILING={failing} \
             MUTATION_FAILURE_DETAIL={detail} mutation_gradle_report",
            reports = reports.display(),
            failing = failing.display(),
            detail = detail.display(),
        ))
        .current_dir(repo_root())
        .output()
        .expect("run mutation_gradle_report");
    assert!(
        out.status.success(),
        "mutation_gradle_report exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read_to_string(&detail).expect("read the detail file")
}

#[test]
fn the_jvm_account_comes_out_of_the_junit_element() {
    // The JVM is the one runner here whose console is not worth parsing —
    // Gradle prints `4 tests completed, 1 failed` and points at an HTML report
    // — so the `<failure>` element IS the account. JUnit splits it in two: the
    // assertion is an attribute and the stack trace is the element's text, and
    // a reader needs both.
    let detail = gradle_report_detail(
        "<testsuite tests=\"2\" failures=\"1\" errors=\"0\" skipped=\"0\">\
           <testcase classname=\"com.sce.GreenTest\" name=\"stillGreen\"/>\
           <testcase classname=\"com.sce.RedTest\" name=\"answersLua\">\
             <failure message=\"expected:&lt;lua&gt; but was:&lt;ecmascript&gt;\" \
                      type=\"org.opentest4j.AssertionFailedError\">\
    at com.sce.RedTest.answersLua(RedTest.kt:42)</failure>\
           </testcase>\
         </testsuite>",
    );
    assert!(
        detail.contains("com.sce.RedTest.answersLua"),
        "the account must be filed under the test it belongs to:\n{detail}"
    );
    assert!(
        detail.contains("expected:<lua> but was:<ecmascript>"),
        "the message attribute is half the account:\n{detail}"
    );
    assert!(
        detail.contains("RedTest.kt:42"),
        "and the element's text is the other half:\n{detail}"
    );
    assert!(
        !detail.contains("stillGreen"),
        "a passing case contributes nothing:\n{detail}"
    );
}

// ── 2. The plumbing ────────────────────────────────────────────────

/// `scripts/mutate` with comment-only and blank lines removed, so a scanner
/// cannot be satisfied by the prose around the code.
fn code_lines() -> Vec<String> {
    let path = repo_root().join("scripts/mutate");
    let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    body.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// The body of a shell function, by name: everything between `name() {` at the
/// start of a line and the closing `}` at the start of a line.
fn function_body(code: &[String], name: &str) -> Vec<String> {
    let opener = format!("{name}() {{");
    let start = code
        .iter()
        .position(|l| l.starts_with(&opener))
        .unwrap_or_else(|| panic!("`scripts/mutate` no longer defines {name}()"));
    let mut body = Vec::new();
    for line in &code[start + 1..] {
        if line.starts_with('}') {
            return body;
        }
        body.push(line.clone());
    }
    panic!("{name}() has no closing brace at the start of a line");
}

/// Every runner the script declares, derived from its own
/// `mutation_declare_runner <name>` calls rather than listed here — so a
/// runner added tomorrow is measured on the day it lands.
fn declared_runners(code: &[String]) -> BTreeSet<String> {
    code.iter()
        .filter(|l| !l.contains("mutation_declare_runner() {"))
        .filter_map(|l| l.split("mutation_declare_runner ").nth(1))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter(|name| name.chars().all(|c| c.is_ascii_lowercase()))
        .map(str::to_string)
        .collect()
}

/// A runner's `_run` function plus the bodies of the `mutation_*` functions it
/// calls directly. One hop, which is what the Gradle runner needs: it
/// truncates the file and delegates the writing to `mutation_gradle_report`.
fn runner_reach(code: &[String], runner: &str) -> Vec<String> {
    let entry = format!("mutation_{runner}_run");
    let body = function_body(code, &entry);
    let mut reach = body.clone();
    let defined: BTreeSet<String> = code
        .iter()
        .filter_map(|l| l.split("() {").next().filter(|n| !n.contains(' ')))
        .filter(|n| n.starts_with("mutation_"))
        .map(str::to_string)
        .collect();
    for callee in &defined {
        if *callee == entry {
            continue;
        }
        if body.iter().any(|l| l.contains(callee.as_str())) {
            reach.extend(function_body(code, callee));
        }
    }
    reach
}

#[test]
fn every_runner_starts_the_account_empty_and_then_writes_it() {
    let code = code_lines();
    // The floor, first: everything below counts occurrences, and zero out of
    // zero lines is the shape that reads as a pass.
    // Measured 2026-08-30: 1460 lines once comments and blanks are dropped.
    // The floor is well under that and well over nothing, so it catches a
    // script that moved, was rewritten in another language, or stopped being
    // read — and does not fail on ordinary growth or ordinary pruning.
    assert!(
        code.len() > 1000,
        "`scripts/mutate` is {} lines of code — the scan has lost its subject",
        code.len()
    );
    let runners = declared_runners(&code);
    assert!(
        runners.len() >= 5,
        "expected the five declared runners, derived {runners:?}"
    );

    for runner in &runners {
        let reach = runner_reach(&code, runner);
        let mentions: Vec<&String> = reach
            .iter()
            .filter(|l| l.contains("MUTATION_FAILURE_DETAIL"))
            .collect();
        // Emptied, so a run that fails nothing cannot inherit the last run's
        // account — the same rule `MUTATION_FAILING` beside it lives under.
        assert!(
            mentions
                .iter()
                .any(|l| l.trim().starts_with(": >") || l.contains("rm -f")),
            "the {runner} runner never empties $MUTATION_FAILURE_DETAIL, so an \
             account could outlive the run it describes"
        );
        // And WRITTEN. Emptying alone would satisfy a scanner while restoring
        // the silence: the file would exist, be empty, and the refusal would
        // print the drift notice for every red baseline forever.
        assert!(
            mentions
                .iter()
                .any(|l| !l.trim().starts_with(": >") && !l.contains("rm -f")),
            "the {runner} runner empties $MUTATION_FAILURE_DETAIL but never writes \
             to it — a red baseline under this runner would report no account"
        );
    }
}

#[test]
fn the_refusal_and_the_retake_both_print_the_account() {
    // The two terminal branches a red run reaches: the baseline that refuses
    // to start a round, and the retake that refuses to re-establish one
    // mid-round. Both listed names and neither had an account.
    let code = code_lines();
    let callers: Vec<&String> = code
        .iter()
        .filter(|l| l.contains("mutation_run_diagnostics"))
        .collect();
    assert!(
        callers.len() >= 3,
        "expected the definition and both callers of mutation_run_diagnostics, \
         found {callers:?}"
    );
    for (function, what) in [
        ("mutation_init", "the baseline refusal"),
        ("mutation_rebaseline", "the retake refusal"),
    ] {
        let body = function_body(&code, function);
        assert!(
            body.iter().any(|l| l.contains("mutation_run_diagnostics")),
            "{what} ({function}) names the failing tests without saying what the \
             runner reported about them"
        );
        // Ordering, because the account is an elaboration of the list: printed
        // first it would read as a separate event.
        let names = body
            .iter()
            .position(|l| l.contains("mutation_baseline_failures"))
            .unwrap_or_else(|| panic!("{function} no longer lists the failing tests"));
        let account = body
            .iter()
            .position(|l| l.contains("mutation_run_diagnostics"))
            .expect("checked above");
        assert!(
            names < account,
            "{what} prints the account before the names it elaborates"
        );
    }
}

// ── 3. The behaviour ───────────────────────────────────────────────

/// A marker no other part of the tree carries, so finding it in the round's
/// output is evidence about this fixture and nothing else.
const WHY_MARKER: &str = "SCE_BASELINE_WHY_MARKER";

const SUBJECT: &str = "def value():\n    return 1\n";

const ORACLE: &str = "import subject\n\n\n\
     def test_value_is_two():\n\
     \x20   assert subject.value() == 2, \"SCE_BASELINE_WHY_MARKER: the subject answered one\"\n";

/// A pytest round whose baseline is red on purpose. Inside the repository under
/// a gitignored path, because `scripts/mutate` derives its root from
/// `git rev-parse --show-toplevel` and the casefile names paths relative to it.
struct Fixture {
    root: PathBuf,
    casefile: PathBuf,
    _ledger: TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture(name: &str) -> Fixture {
    let rel = format!("tmp/baseline-why-{name}");
    let root = repo_root().join(&rel);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the fixture directory");
    fs::write(root.join("subject.py"), SUBJECT).expect("write subject.py");
    fs::write(root.join("test_oracle.py"), ORACLE).expect("write test_oracle.py");

    let casefile = root.join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "mutation_pytest {rel}\n\
             mutation_runtime_targets {rel}/subject.py\n\
             mutation_oracles {rel}/test_oracle.py\n\n\
             mutation_case \"the subject stops answering\" <<'PY'\n\
             edit(\"{rel}/subject.py\", \"return 1\", \"return 0\")\n\
             PY\n"
        ),
    )
    .expect("write the casefile");

    Fixture {
        root,
        casefile,
        _ledger: tempdir().expect("temp ledger"),
    }
}

impl Fixture {
    /// Run the round; return (stdout, stderr) separately, because which stream
    /// the account lands on is part of what this measures.
    ///
    /// `SCE_MUTATION_LEDGER_DIR` is what keeps this round out of the real
    /// corpus, and it is the whole of the isolation needed: `$HOME` reaches
    /// `scripts/lib/mutation_ledger.sh` only as the DEFAULT root that variable
    /// overrides. Overriding `$HOME` as well — a second lock on the same door —
    /// breaks the runner instead: measured 2026-08-30, pytest lives in
    /// `~/.local/lib/python3.12/site-packages` on this machine, so a temporary
    /// home turns the round into `No module named pytest` and the fixture stops
    /// measuring anything.
    fn round(&self) -> (String, String) {
        let out = Command::new(repo_root().join("scripts/mutate"))
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("SCE_MUTATION_LEDGER_DIR", self._ledger.path())
            .output()
            .expect("run scripts/mutate");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

#[test]
fn a_red_baseline_prints_the_runners_account_on_stdout() {
    if Command::new("python3")
        .args(["-c", "import pytest"])
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("SKIP: pytest unavailable");
        return;
    }
    let f = fixture("account");
    let (stdout, stderr) = f.round();

    // The refusal itself, unchanged.
    assert!(
        stdout.contains("baseline is not green (1 failing)"),
        "the round did not refuse the way this fixture expects:\n{stdout}{stderr}"
    );
    // The name, which the refusal already carried.
    assert!(
        stdout.contains("test_oracle.py::test_value_is_two"),
        "the refusal stopped naming the failing test:\n{stdout}"
    );
    // And the account, which is the repair. On STDOUT: the ledger keeps a
    // round's own stdout as its console log, so an account on the other stream
    // is one a later reader of the record cannot find.
    assert!(
        stdout.contains(WHY_MARKER),
        "the refusal named the test but not why it failed — the runner's \
         account was dropped again:\n{stdout}\n--- stderr ---\n{stderr}"
    );
    assert!(
        stdout.contains("assert 1 == 2"),
        "pytest's own comparison must survive into the refusal:\n{stdout}"
    );
    // Order: the account elaborates the list, so it comes after it.
    let name_at = stdout
        .find("test_oracle.py::test_value_is_two")
        .expect("checked above");
    let why_at = stdout.find(WHY_MARKER).expect("checked above");
    assert!(
        name_at < why_at,
        "the account must follow the names it elaborates:\n{stdout}"
    );
}
