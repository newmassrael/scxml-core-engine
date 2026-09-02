// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An empty arm is not a green row, and here that is a case rather than a
//! sentence.
//!
//! `scripts/gates/w3c-kotlin.sh` runs the Kotlin conformance suite once per
//! engine/language pair and reads each row's verdict from the JUnit XML the
//! run produced, because Gradle's exit status cannot carry it. **Measured
//! 2026-09-02**, by narrowing one row's test filter to a class that does not
//! exist through an `--init-script`:
//!
//! ```text
//! > Task :sce-kotlin-tests:test
//! BUILD SUCCESSFUL in 15s
//! ```
//!
//! Zero cases ran, Gradle reported success, and `backends/kotlin/tests/build/
//! test-results/test` was left present holding `binary/` and no `TEST-*.xml`
//! at all. So the totals a reader takes off that directory — cases, failures,
//! errors, skips — are all 0, and every threshold held over one of them is
//! satisfied by the run that measured nothing. The row was refused all the
//! same, by the comparison against the class set the suite's own sources
//! declare: `251 test class(es) ... produced no JUnit report`.
//!
//! ## Why the refusal moved into a program to be tested here
//!
//! That measurement had to be bought BY HAND, and so did the five before it
//! (`no_shell_runner_reaches_a_gates_own_logic`): the corpus has runners for
//! cargo, ctest, go, pytest and gradle, and none of them can drive a gate
//! script's own logic, because the input of this particular verdict — a
//! directory of JUnit reports — does not exist until the gate has run Gradle.
//! A test that produced it would have to run the gate, and would then be the
//! gate.
//!
//! A directory of JUnit reports is, however, just a directory of files, and
//! nothing says it has to come from Gradle. `scripts/gates/kotlin_coverage.py`
//! is that verdict as a program, and this suite hands it directories built
//! here: the empty arm, the complete one, the one that dropped a class, the
//! one that reported a class nobody can account for, the skipped case. What
//! the gate keeps is the delegation — held by `gate_registry_contract`'s
//! `the_kotlin_gate_runs_every_engine_it_claims`, which reads the row loop and
//! fails if the verdict stops being asked for inside it.
//!
//! ⚠ A refusal is exit code 3, and every case here asserts that number rather
//! than "not zero". A reader that died on its way to a decision also exits
//! non-zero, and a suite that cannot tell the two apart would report a
//! crashing verdict as a working one — which is the same defect, in the
//! exit-status alphabet, as reading a quiet zero as an answer.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A decision this reader made, and the answer is no.
const REFUSED: i32 = 3;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn coverage_program() -> PathBuf {
    let program = repo_root().join("scripts/gates/kotlin_coverage.py");
    assert!(
        program.is_file(),
        "no coverage program at {} — `scripts/gates/w3c-kotlin.sh` asks it for \
         every row's verdict, so its absence is a gate that judges nothing",
        program.display()
    );
    program
}

struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Outcome {
    let out = Command::new("python3")
        .arg(coverage_program())
        .args(args)
        .output()
        .expect("python3 runs the coverage program");
    Outcome {
        code: out
            .status
            .code()
            .expect("the program exited rather than signalled"),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// One `<testsuite>` as Gradle writes it, with the cases spelled out.
///
/// `skipped` names are written as real `<testcase><skipped/></testcase>`
/// elements rather than only bumping the attribute, because the verdict names
/// the skipped cases and a count alone would let it pass with an empty list.
///
/// ⚠ The `<testsuite name=...>` here is a DISPLAY name that deliberately
/// differs from the class, because that is what Gradle writes and the
/// difference is not cosmetic. Measured 2026-09-02 on the real suite:
/// `TEST-com.sce.w3c.Test453.xml` carries `name="Test 453 -- W3C SCXML B.2"`
/// and `classname="com.sce.w3c.Test453"`. A reader that took the `name`
/// reported 242 of 251 classes as never having run over a suite that ran in
/// full — so every fixture here carries the trap, and a verdict that goes back
/// to reading `name` fails all of them rather than none.
fn write_report(reports: &Path, class: &str, passing: usize, failing: usize, skipped: &[&str]) {
    let total = passing + failing + skipped.len();
    let mut body = String::new();
    for index in 0..passing {
        body.push_str(&format!(
            "  <testcase classname=\"{class}\" name=\"passes{index}\"/>\n"
        ));
    }
    for index in 0..failing {
        body.push_str(&format!(
            "  <testcase classname=\"{class}\" name=\"fails{index}\">\
             <failure message=\"no\"/></testcase>\n"
        ));
    }
    for name in skipped {
        body.push_str(&format!(
            "  <testcase classname=\"{class}\" name=\"{name}\"><skipped/></testcase>\n"
        ));
    }
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <testsuite name=\"a display name, not {class}\" tests=\"{total}\" \
         failures=\"{failing}\" errors=\"0\" skipped=\"{}\">\n{body}</testsuite>\n",
        skipped.len()
    );
    fs::write(reports.join(format!("TEST-{class}.xml")), xml).expect("the report is writable");
}

/// A suite of `count` classes, reported in full and passing.
struct Arm {
    _dir: tempfile::TempDir,
    reports: PathBuf,
    runnable: PathBuf,
    classes: Vec<String>,
}

fn arm(count: usize) -> Arm {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let reports = dir.path().join("test-results/test");
    fs::create_dir_all(&reports).expect("the report directory is creatable");
    let classes: Vec<String> = (0..count)
        .map(|index| format!("com.sce.integration.Synthetic{index}Test"))
        .collect();
    for class in &classes {
        write_report(&reports, class, 3, 0, &[]);
    }
    let runnable = dir.path().join("runnable-classes.txt");
    fs::write(&runnable, format!("{}\n", classes.join("\n"))).expect("the set is writable");
    Arm {
        _dir: dir,
        reports,
        runnable,
        classes,
    }
}

fn verdict(arm: &Arm) -> Outcome {
    run(&[
        "verdict",
        "--reports",
        arm.reports.to_str().expect("utf-8 path"),
        "--runnable",
        arm.runnable.to_str().expect("utf-8 path"),
        "--label",
        "rhino over ecmascript machines",
    ])
}

/// ⚠ THE CASE THIS FILE EXISTS FOR.
///
/// The arm ran nothing: Gradle succeeded, the report directory is present and
/// holds no report. Every total is 0 and every count-based threshold is
/// therefore met, so what must refuse it is the comparison against the classes
/// the suite declares — and it must name them, because "0 cases" says a run
/// happened and measured nothing while "these classes never reported" says
/// which claims went unmade.
#[test]
fn an_arm_that_ran_nothing_is_refused() {
    let arm = arm(4);
    for report in fs::read_dir(&arm.reports).expect("the report directory is readable") {
        fs::remove_file(report.expect("an entry").path()).expect("the report is removable");
    }

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "an arm that reported no class at all was not refused (exit {}). Gradle \
         reports BUILD SUCCESSFUL over a test task that ran nothing — measured \
         2026-09-02 — so this verdict is the only thing between an empty run \
         and a green row.\nstdout: {}\nstderr: {}",
        out.code, out.stdout, out.stderr
    );
    for class in &arm.classes {
        assert!(
            out.stderr.contains(class),
            "the refusal does not name `{class}`, which never reported. A count \
             says a run measured nothing; the names say which conformance \
             claims went unmade.\nstderr: {}",
            out.stderr
        );
    }
}

/// The control: the same reader accepts the arm that ran everything.
///
/// Without it every case here would pass against a program that refuses
/// unconditionally, which measures the refusal and nothing about the
/// distinction it draws.
#[test]
fn an_arm_that_reported_every_class_is_accepted() {
    let arm = arm(4);
    let out = verdict(&arm);
    assert_eq!(
        out.code, 0,
        "a complete arm was refused.\nstdout: {}\nstderr: {}",
        out.stdout, out.stderr
    );
    assert_eq!(
        out.stdout.trim(),
        "12",
        "the accepted arm must report the case total it counted — the gate \
         prints it, and a verdict that returned nothing would leave the row's \
         summary claiming a number nobody read.\nstdout: {}",
        out.stdout
    );
}

/// One class stops running. The shape a case-count floor could not see: the
/// suite reports 3 fewer cases out of a suite whose total sat well above any
/// floor a reader would dare write down.
#[test]
fn a_class_that_stopped_reporting_is_refused() {
    let arm = arm(4);
    let dropped = &arm.classes[2];
    fs::remove_file(arm.reports.join(format!("TEST-{dropped}.xml")))
        .expect("the report is removable");

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a row that silently stopped running one class was accepted.\nstderr: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains(dropped),
        "the refusal does not name the class that stopped reporting.\nstderr: {}",
        out.stderr
    );
}

/// The other direction, and it fails on a DIFFERENT defect: a class in the
/// report that the derivation cannot account for is this reader going blind —
/// an annotation it does not know, a supertype clause it could not parse.
/// Checking only the first direction would let a blind reader report a
/// shrinking suite as whole.
#[test]
fn a_class_the_derivation_cannot_account_for_is_refused() {
    let arm = arm(4);
    write_report(
        &arm.reports,
        "com.sce.integration.UnaccountedTest",
        2,
        0,
        &[],
    );

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a row reporting a class outside the derived set was accepted — \
         unclassified is RED, not a bonus.\nstderr: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains("UnaccountedTest"),
        "the refusal does not name the unaccounted class.\nstderr: {}",
        out.stderr
    );
}

/// The method-level half. The class comparison cannot see an `@Disabled` on a
/// method — the class still reports — so a skip is asked separately, and by
/// NAME: a count says how many stopped being measured without saying which.
#[test]
fn a_skipped_case_is_refused() {
    let arm = arm(4);
    let class = arm.classes[1].clone();
    write_report(&arm.reports, &class, 2, 0, &["theCaseNobodyRan"]);

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a row carrying a skipped case was accepted. A skipped case is measured \
         by nothing — a conformance claim the row did not make.\nstderr: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains("theCaseNobodyRan"),
        "the refusal does not name the skipped case.\nstderr: {}",
        out.stderr
    );
}

#[test]
fn a_failing_case_is_refused() {
    let arm = arm(4);
    let class = arm.classes[0].clone();
    write_report(&arm.reports, &class, 2, 1, &[]);

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a row with a failing case was accepted.\nstderr: {}",
        out.stderr
    );
}

/// ⚠ The vacuity guard, on the side the comparison cannot defend.
///
/// The comparison is an equality and two empty sets are equal, so a derivation
/// that parsed nothing would accept every row — and the row that would expose
/// it is the one that ran nothing, which is precisely the row this verdict
/// exists to refuse. The floor therefore sits under the DERIVATION, and the
/// verdict refuses an empty set outright rather than matching it.
#[test]
fn an_empty_runnable_set_refuses_rather_than_matching_an_empty_arm() {
    let arm = arm(4);
    for report in fs::read_dir(&arm.reports).expect("the report directory is readable") {
        fs::remove_file(report.expect("an entry").path()).expect("the report is removable");
    }
    fs::write(&arm.runnable, "").expect("the set is writable");

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "an empty class set matched an empty report and the row passed. Two \
         empty sets are equal, which is why the emptiness has to be refused \
         rather than compared.\nstdout: {}\nstderr: {}",
        out.stdout, out.stderr
    );
}

#[test]
fn a_missing_report_directory_is_refused() {
    let arm = arm(4);
    fs::remove_dir_all(&arm.reports).expect("the report directory is removable");

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a run that produced no result directory at all was accepted.\nstderr: {}",
        out.stderr
    );
}

/// An unreadable report is refused as unreadable, not absorbed as a class that
/// never reported. Both are red, but they name different repairs.
#[test]
fn a_report_this_reader_cannot_parse_is_refused_as_unreadable() {
    let arm = arm(4);
    fs::write(
        arm.reports
            .join("TEST-com.sce.integration.Synthetic0Test.xml"),
        "<testsuite name=\"truncated\"",
    )
    .expect("the report is writable");

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a malformed JUnit report was accepted.\nstderr: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains("could not be read"),
        "an unreadable report was reported as something other than unreadable, \
         which sends a reader to the wrong repair.\nstderr: {}",
        out.stderr
    );
}

/// The file name is this reader's index into the report, so it is CHECKED
/// against the cases inside rather than trusted. A report whose name does not
/// describe its content would otherwise put one class in the reported set on
/// another class's evidence — silent in both directions at once, since the
/// named class looks present and the running one looks absent.
#[test]
fn a_report_whose_name_does_not_describe_its_cases_is_refused() {
    let arm = arm(4);
    let claimed = &arm.classes[0];
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <testsuite name=\"a display name\" tests=\"1\" failures=\"0\" errors=\"0\" \
         skipped=\"0\">\n  \
         <testcase classname=\"com.sce.integration.SomeOtherTest\" name=\"passes\"/>\n\
         </testsuite>\n"
    );
    fs::write(arm.reports.join(format!("TEST-{claimed}.xml")), xml)
        .expect("the report is writable");

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a report filed under one class and carrying another's cases was \
         accepted.\nstderr: {}",
        out.stderr
    );
}

/// A report holding no case at all leaves its class identifiable only by the
/// file name, which is the one thing this reader refuses to take on trust.
#[test]
fn a_report_with_no_case_in_it_is_refused() {
    let arm = arm(4);
    let claimed = &arm.classes[0];
    fs::write(
        arm.reports.join(format!("TEST-{claimed}.xml")),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <testsuite name=\"a display name\" tests=\"0\" failures=\"0\" errors=\"0\" \
         skipped=\"0\"/>\n",
    )
    .expect("the report is writable");

    let out = verdict(&arm);
    assert_eq!(
        out.code, REFUSED,
        "a class reported a result file with no case in it and the row \
         passed.\nstderr: {}",
        out.stderr
    );
}

/// A synthetic Kotlin suite: `filler` concrete classes that each declare a
/// case, plus whatever `extra` sources the case under test needs.
///
/// The filler is not padding for its own sake — the derivation refuses a set
/// under its floor, and that floor is unconditional on purpose. A `--floor`
/// flag for tests would be an escape hatch out of the one guard standing
/// between a reader that parsed nothing and a gate that passes every row.
fn synthetic_sources(filler: usize, extra: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let sources = dir.path().join("com/sce/integration");
    fs::create_dir_all(&sources).expect("the source directory is creatable");
    for index in 0..filler {
        fs::write(
            sources.join(format!("Filler{index}Test.kt")),
            format!(
                "package com.sce.integration\n\n\
                 class Filler{index}Test {{\n    @Test\n    fun runs() {{}}\n}}\n"
            ),
        )
        .expect("the filler source is writable");
    }
    for (name, body) in extra {
        fs::write(sources.join(name), body).expect("the source is writable");
    }
    dir
}

fn derive(sources: &Path) -> Outcome {
    run(&["derive", sources.to_str().expect("utf-8 path")])
}

fn derived_set(out: &Outcome) -> BTreeSet<String> {
    out.stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// The fixpoint, which is the reason the derivation is not a grep for `@Test`:
/// a generated case is `class Test144 : W3CTestBase<…>()` and declares no
/// annotation of its own. A reader that only counted annotated types would
/// call the whole W3C suite unrunnable and then find every row complete.
#[test]
fn the_derivation_follows_inheritance_rather_than_the_annotation() {
    let tree = synthetic_sources(
        200,
        &[(
            "Inherited.kt",
            "package com.sce.integration\n\n\
             abstract class SyntheticBase {\n    @Test\n    fun inheritedCase() {}\n}\n\n\
             class SyntheticChild : SyntheticBase()\n\n\
             interface SyntheticMarker\n",
        )],
    );

    let out = derive(tree.path());
    assert_eq!(
        out.code, 0,
        "the derivation refused a readable tree.\nstderr: {}",
        out.stderr
    );
    let derived = derived_set(&out);

    assert!(
        derived.contains("com.sce.integration.SyntheticChild"),
        "a concrete class that inherits its cases was not derived, so the \
         generated W3C cases — which declare no annotation of their own — \
         would go unaccounted.\nderived: {derived:?}"
    );
    assert!(
        !derived.contains("com.sce.integration.SyntheticBase"),
        "an abstract base was derived. JUnit never reports one, so a row would \
         be refused for a class that cannot run.\nderived: {derived:?}"
    );
    assert!(
        !derived.contains("com.sce.integration.SyntheticMarker"),
        "an interface was derived, and JUnit does not report interfaces \
         either.\nderived: {derived:?}"
    );
}

/// ⚠ Comments are stripped before anything is read. This repository has
/// already watched a scanner read its own prose — `reach_of` matched a gate
/// script's COMMENT and demanded a tool that lane never installed — and a
/// `@Test` or a `class` named in a KDoc is exactly that defect here: it would
/// put a class that does not exist into the set every row must report, and
/// every row would then be red for a reason no repair can reach.
#[test]
fn the_derivation_does_not_read_its_own_prose() {
    let tree = synthetic_sources(
        200,
        &[(
            "Commented.kt",
            "package com.sce.integration\n\n\
             // class CommentedOutTest {\n// @Test\n// fun runs() {}\n// }\n\n\
             /*\nclass BlockCommentedTest {\n    @Test\n    fun runs() {}\n}\n*/\n",
        )],
    );

    let out = derive(tree.path());
    assert_eq!(
        out.code, 0,
        "the derivation refused a readable tree.\nstderr: {}",
        out.stderr
    );
    let derived = derived_set(&out);

    for ghost in ["CommentedOutTest", "BlockCommentedTest"] {
        assert!(
            !derived.iter().any(|name| name.ends_with(ghost)),
            "`{ghost}` is written only inside a comment and was derived anyway. \
             Every row would then be refused for a class that does not \
             exist.\nderived: {derived:?}"
        );
    }
}

/// A derivation that read nothing REFUSES, and does not hand the rows an empty
/// set to match. The quiet zero this repository keeps re-learning: a reader
/// that parsed nothing reports every row as complete.
#[test]
fn the_derivation_refuses_a_tree_it_could_not_read() {
    let tree = tempfile::tempdir().expect("a scratch directory");

    let out = derive(tree.path());
    assert_eq!(
        out.code, REFUSED,
        "a source tree with no test class in it was accepted, so every row \
         below would compare its report against an empty set and \
         pass.\nstdout: {}\nstderr: {}",
        out.stdout, out.stderr
    );

    let missing = tree.path().join("not-a-directory");
    let out = derive(&missing);
    assert_eq!(
        out.code, REFUSED,
        "a path that is not a directory was accepted as a source tree.\nstderr: {}",
        out.stderr
    );
}

/// The gate's own header names two classes as "what a reader can act on":
/// `SendParamPayloadTest` (W3C SCXML 6.2, a repeated `<param>` name) and
/// `XmlDataIsADomTreeTest` (W3C SCXML B.2, a `<data>` element's XML arriving
/// as a document) — the pair the Lua engine failed on 2026-08-29 and passes
/// since `sce-build`'s frontend was linked into it.
///
/// A name in a comment is not a lane. This asks the derivation for them, on
/// the real sources, so the sentence is answered by the tree instead of by a
/// reader trusting it.
#[test]
fn the_derivation_names_the_two_classes_the_gates_header_calls_actionable() {
    let sources = repo_root().join("backends/kotlin/tests/src/test/kotlin");
    assert!(
        sources.is_dir(),
        "no Kotlin test sources at {} — the gate derives the classes every row \
         must report from exactly this tree",
        sources.display()
    );

    let out = derive(&sources);
    assert_eq!(
        out.code, 0,
        "the derivation refused the committed Kotlin test sources.\nstderr: {}",
        out.stderr
    );
    let derived = derived_set(&out);

    for named in ["SendParamPayloadTest", "XmlDataIsADomTreeTest"] {
        assert!(
            derived.iter().any(|class| class.ends_with(named)),
            "`{named}` is named in the Kotlin gate's header as a case a reader \
             can act on, and the derivation does not produce it — so no row is \
             held to running it. Either the class was renamed, in which case \
             the header and this case move with it, or the fixpoint stopped \
             reaching it, which is the blind reader the two-way comparison \
             exists to catch."
        );
    }
}
