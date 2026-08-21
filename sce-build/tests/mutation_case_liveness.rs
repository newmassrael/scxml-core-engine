// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The mutation harness tells the truth about its own verdicts.
//
// Two halves. `scripts/mutate --check` refuses the shapes a dead mutation
// case takes, and the parsers in `scripts/lib/mutation_failures.sh` name the
// tests a CAUGHT verdict is made of. Both are checked here against inputs
// built to break them, because both are the kind of code whose failure looks
// exactly like success: a refusal that stopped refusing reads as a clean
// corpus, and a parser that stopped matching reads as a verdict with nothing
// to attribute.
//
// The gate `mutation-cases` runs that mode over every casefile in
// `sce-build/tests/mutations/` on every push, and a gate is only worth its
// runtime if it can turn red. This file is where that is established: each
// case below hands the harness a casefile built to be broken in exactly one
// way and requires a non-zero exit naming that way, plus one built to be
// sound and required to pass.
//
// The shapes are not invented. Every one of them was found in the corpus the
// first time the mode ran:
//
//   - an anchor that matches several places, so `edit()`'s "replace the
//     first" was aiming at whichever backend happened to appear earliest in
//     `sce_codegen.rs` — three cases in two files, one of them the pair whose
//     anchor closed two `format!` calls where the cases meant one each;
//   - a selector whose `--lib` this check first rejected outright, because
//     cargo reports a library target under its crate type (`rlib`) rather
//     than under the word `lib`. That was a defect in the checker, and it is
//     pinned below so the checker cannot reacquire it.
//
// The fixtures are self-contained: each declares a target inside its own
// temporary directory, so nothing here edits the repository, and a run that
// dies half way cannot leave a mutation in the tree.

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

/// The line every fixture uses to name a suite that exists.
///
/// A real selector, because the mode resolves it: `cargo metadata` has to
/// find the package, the feature and the target. A fixture naming an
/// imaginary one would fail for that reason instead of the reason it is
/// written to test.
const LIVE_SELECTOR: &str = "mutation_tests -p sce-build --features cli --lib";

struct Fixture {
    _dir: TempDir,
    casefile: PathBuf,
    target: PathBuf,
}

/// A casefile and the file it studies, in a directory of their own.
///
/// `target_body` is written verbatim, and `cases` is appended to the
/// declarations, so a test spells only what it is about. `TARGET` in a case
/// body stands for the subject's path — the fixture knows it and the test
/// should not have to.
fn fixture(target_body: &str, cases: &str) -> Fixture {
    fixture_with_selector(LIVE_SELECTOR, target_body, cases)
}

fn fixture_with_selector(selector: &str, target_body: &str, cases: &str) -> Fixture {
    let dir = tempdir().expect("temp dir");
    let target = dir.path().join("subject.txt");
    fs::write(&target, target_body).expect("write the subject");

    // The oracle side of the declaration, which the harness requires of every
    // casefile: a selector says which suite runs, never which file holds the
    // assertions, so a casefile that names none leaves its verdicts unwatched
    // when that file is weakened. A `--lib` selector — what these fixtures use
    // — derives nothing, so it is spelled. A stand-in file rather than the
    // subject, because naming the subject would make the two sides of the
    // verdict one file and quietly rob every test below of the distinction.
    let oracle = dir.path().join("oracle.txt");
    fs::write(&oracle, "the assertions that would catch it\n").expect("write the oracle");
    let oracle_path = oracle.display().to_string();

    let path = target.display().to_string();
    let casefile = dir.path().join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "{selector}\nmutation_targets {path}\nmutation_oracles {oracle_path}\n\n{}",
            // `{:?}` on the path yields a quoted, escaped literal, which is
            // what a Python case body needs to name it.
            cases.replace("TARGET", &format!("{path:?}"))
        ),
    )
    .expect("write the casefile");

    Fixture {
        _dir: dir,
        casefile,
        target,
    }
}

/// Run the check mode over a fixture; return (success, combined output).
fn check(casefile: &Path) -> (bool, String) {
    let out = Command::new(repo_root().join("scripts/mutate"))
        .arg("--check")
        .arg(casefile)
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --check");

    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn assert_rejected(fixture: &Fixture, expected: &str) {
    let (ok, output) = check(&fixture.casefile);
    assert!(
        !ok,
        "the check mode accepted a casefile that cannot test anything:\n{output}"
    );
    assert!(
        output.contains(expected),
        "rejected, but not for the reason under test — expected {expected:?} in:\n{output}"
    );
}

#[test]
fn a_case_whose_anchor_is_gone_is_refused() {
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the anchor moved\" <<'PY'\n\
         edit(TARGET, \"x + 2\", \"x + 3\")\n\
         PY\n",
    );
    assert_rejected(&f, "anchor not found");
}

#[test]
fn a_case_whose_anchor_matches_twice_is_refused() {
    // The shape that cost three cases in the corpus: four backends spelled
    // one predicate identically, and the case mutated whichever came first.
    let f = fixture(
        "fn a() -> bool {\n    ready()\n}\n\nfn b() -> bool {\n    ready()\n}\n",
        "mutation_case \"which one does this mean\" <<'PY'\n\
         edit(TARGET, \"    ready()\", \"    false\")\n\
         PY\n",
    );
    assert_rejected(&f, "anchor matches 2 places");
}

#[test]
fn a_case_that_replaces_its_anchor_with_itself_is_refused() {
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"a mutation that mutates nothing\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x + 1\")\n\
         PY\n",
    );
    assert_rejected(&f, "replaces its anchor with itself");
}

#[test]
fn a_casefile_with_no_cases_is_refused() {
    // The emptiest form of the failure: declares a suite and a target, runs
    // clean, tests nothing.
    let f = fixture("fn keep() {}\n", "");
    assert_rejected(&f, "declared no mutation_case");
}

#[test]
fn a_declared_target_that_does_not_exist_is_refused() {
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"studies a file that is gone\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x + 2\")\n\
         PY\n",
    );
    fs::remove_file(&f.target).expect("remove the subject");
    assert_rejected(&f, "declared target does not exist");
}

#[test]
fn a_selector_naming_a_suite_that_does_not_exist_is_refused() {
    let f = fixture_with_selector(
        "mutation_tests -p sce-build --features cli --test no_such_suite_exists",
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the suite was renamed out from under it\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x + 2\")\n\
         PY\n",
    );
    assert_rejected(&f, "names no test target");
}

// ── A template target, and the step between it and the binaries ──

/// A codegen template that is in the tree, named the way a casefile names it.
///
/// A real path, because the refusal under test is about where the file lives:
/// the fixtures above write their subject into a temp directory, and a temp
/// path is not under `tools/codegen/templates/` however it is spelled. The
/// refusal fires before the snapshot, so nothing in the working tree is read
/// or written by these two tests.
const A_TEMPLATE: &str = "tools/codegen/templates/rust/invoke_methods.rs.jinja2";

/// A selector whose artifacts do NOT embed the templates.
///
/// `sce-build` writes the template tree into its own binaries through its
/// build script, so a round selecting one of ITS tests reaches a template
/// with nothing declared — which is what the seven casefiles that mutate
/// templates in a cargo round rely on. `sce-rust-tests` links no such crate
/// (measured through `cargo metadata`: its dependency closure does not
/// contain `sce-build`), so a template mutation reaches its binaries only
/// through the committed generated tree, and only if something regenerates
/// it. That is the shape these two tests are about.
const SELECTOR_WITHOUT_THE_TEMPLATES: &str = "mutation_tests -p sce-rust-tests --test ai_loop";

/// A casefile declaring the temp subject AND a real template, with whatever
/// extra declarations the test is about.
fn fixture_with_template(extra: &str) -> Fixture {
    fixture_with_template_under(SELECTOR_WITHOUT_THE_TEMPLATES, extra)
}

fn fixture_with_template_under(selector: &str, extra: &str) -> Fixture {
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
            "{selector}\n\
             mutation_targets {path}\n\
             mutation_targets {A_TEMPLATE}\n\
             mutation_oracles {}\n\
             {extra}\n\n\
             mutation_case \"studies the generated code\" <<'PY'\n\
             edit({path:?}, \"x + 1\", \"x + 2\")\n\
             PY\n",
            oracle.display()
        ),
    )
    .expect("write the casefile");

    Fixture {
        _dir: dir,
        casefile,
        target,
    }
}

/// A cargo round cannot reach a template it does not regenerate.
///
/// `cargo test --no-run` compiles the committed generated tree; the codegen
/// step that writes that tree is not part of it. Measured on CI 2026-08-21:
/// both cases of `invoke_param_is_a_value_not_source.cases` reported
/// INCONCLUSIVE — "the mutation never reached the code under test" — for
/// exactly this reason, days after the round that wrote them read as evidence.
#[test]
fn a_cargo_casefile_that_mutates_a_template_without_regenerating_it_is_refused() {
    let f = fixture_with_template("");
    assert_rejected(
        &f,
        "a codegen template is declared as a target of a cargo round",
    );
    let (_, output) = check(&f.casefile);
    assert!(
        output.contains(A_TEMPLATE),
        "the refusal did not name the template it is about:\n{output}"
    );
}

/// And the same casefile is accepted once the step is declared.
///
/// The other half of the measurement: a check that only ever refuses is
/// indistinguishable from one that refuses everything, and this suite's whole
/// subject is a mode whose verdicts nothing else re-proves.
#[test]
fn a_cargo_casefile_that_declares_the_regeneration_is_accepted() {
    let f = fixture_with_template("mutation_regen true");
    let (ok, output) = check(&f.casefile);
    assert!(
        ok,
        "a template target with its regeneration declared was refused:\n{output}"
    );
}

/// And a round whose binaries CONTAIN the templates needs no declaration.
///
/// The third reading, and the one that decides whether the refusal above is
/// a rule or a nuisance: seven casefiles in the corpus mutate a template
/// under a selector that reaches `sce-build`, whose build script writes the
/// template tree into the artifact. Rebuilding it is the bridge. A check that
/// could not tell those from the inert one would have to be switched off.
#[test]
fn a_cargo_casefile_whose_binaries_embed_the_templates_needs_no_regeneration() {
    let f = fixture_with_template_under(LIVE_SELECTOR, "");
    let (ok, output) = check(&f.casefile);
    assert!(
        ok,
        "a template target was refused under a selector that embeds it:\n{output}"
    );
}

// ── The ctest selector, and the tree it is resolved against ──────

/// Whether `ninja` will answer for a directory at all.
fn ninja_available() -> bool {
    Command::new("ninja")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A directory that looks configured to the harness, with a `build.ninja`
/// whose generator would or would not re-run cmake.
///
/// `current` writes a build.ninja with nothing out of date; the other
/// writes one whose only edge is out of date, which is the shape a tree
/// takes when a `CMakeLists.txt` moved after the last configure.
fn fake_build_tree(dir: &Path, current: bool) {
    fs::create_dir_all(dir).expect("create the build dir");
    fs::write(dir.join("CMakeCache.txt"), "// a configured tree\n").expect("write the cache");
    fs::write(dir.join("CMakeLists.txt"), "# an input\n").expect("write the input");
    fs::write(
        dir.join("build.ninja"),
        "rule RERUN_CMAKE\n  command = true\n  description = Re-running CMake...\n\
         build build.ninja: RERUN_CMAKE CMakeLists.txt\n",
    )
    .expect("write the generator file");
    // Ninja decides from mtimes, so the two trees differ in exactly that.
    // A configured-then-edited tree has an input newer than the generator
    // file; a freshly configured one has the generator file newer. Written
    // rather than slept for: a test that waits on the clock is a test that
    // is flaky on a fast filesystem and slow on every one.
    let stamp = std::time::SystemTime::now();
    let (generator_age, input_age) = if current {
        (stamp, stamp - std::time::Duration::from_secs(60))
    } else {
        (stamp - std::time::Duration::from_secs(60), stamp)
    };
    for (name, age) in [
        ("build.ninja", generator_age),
        ("CMakeLists.txt", input_age),
    ] {
        let file = fs::File::options()
            .write(true)
            .open(dir.join(name))
            .expect("open for stamping");
        file.set_times(fs::FileTimes::new().set_modified(age))
            .expect("stamp the file");
    }
    if current {
        // Mtimes are not the whole of ninja's answer: it also remembers
        // the command each edge last ran, and an edge it has never run is
        // dirty however new its output is. A configured tree has been
        // built at least once, so the fixture is too — measured, after
        // the first version of this test wrote the files and expected
        // ninja to call them current.
        Command::new("ninja")
            .args(["-C", dir.to_str().expect("utf-8 path"), "build.ninja"])
            .output()
            .expect("run the fixture's generator once");
    }
}

/// A tree that is configured but not current cannot condemn a casefile.
///
/// The defect this pins was measured on the build machine: its `build/`
/// was configured three days before the target a casefile names, so the
/// selector matched nothing and the gate reported the casefile as one
/// that "no longer applies to the tree" — a verdict about the author's
/// text drawn from the checker's own stale input. The same happens to
/// anyone who adds a ctest target and runs the gate before re-running
/// cmake.
#[test]
fn a_configured_tree_that_predates_the_casefile_cannot_condemn_it() {
    if !ninja_available() {
        eprintln!("SKIP: ninja unavailable");
        return;
    }
    let dir = tempdir().expect("temp dir");
    let build = dir.path().join("build");
    fake_build_tree(&build, false);

    let f = fixture_with_selector(
        &format!(
            "mutation_ctest --test-dir {} -R ^NoSuchTestIsRegistered$",
            build.display()
        ),
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the arithmetic is wrong\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );

    let (ok, output) = check(&f.casefile);
    assert!(
        ok,
        "a tree whose test list predates the casefile was read as a dead case:\n{output}"
    );
    assert!(
        output.contains("configured but not current"),
        "the skip did not say why it could not judge:\n{output}"
    );
}

/// A tree that has only drifted still checks a selector that matches.
///
/// The order the check asks its two questions in is the whole of this:
/// staleness matters only when nothing matched. A developer's tree is
/// drifted most of the time — a commit moves a CMake input and the
/// generator would re-run — and it is still right about every test that
/// existed before it. Asking about staleness first cost every ctest
/// selector its check on such a tree, which is a coverage loss traded
/// for a false red, measured on this repository's own `build/`.
#[test]
fn a_live_selector_is_still_checked_on_a_tree_that_has_only_drifted() {
    if !ninja_available() {
        eprintln!("SKIP: ninja unavailable");
        return;
    }
    let dir = tempdir().expect("temp dir");
    let build = dir.path().join("build");
    fake_build_tree(&build, false);
    // A test registry of one, which is what makes this tree drifted
    // rather than ignorant: it knows the test the selector names.
    fs::write(
        build.join("CTestTestfile.cmake"),
        "add_test(fixture_test \"/bin/true\")\n",
    )
    .expect("write the registry");

    let f = fixture_with_selector(
        &format!(
            "mutation_ctest --test-dir {} -R ^fixture_test$",
            build.display()
        ),
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the arithmetic is wrong\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );

    let (ok, output) = check(&f.casefile);
    assert!(ok, "a live selector was refused:\n{output}");
    assert!(
        output.contains("selector matches 1 registered test"),
        "the selector went unchecked on a tree that knows it:\n{output}"
    );
}

/// A tree that *is* current still condemns a selector matching nothing.
///
/// The other half, and the one that keeps the check worth running: the
/// discriminator above must not become a way for every ctest casefile to
/// go unchecked.
#[test]
fn a_current_tree_still_refuses_a_selector_that_matches_nothing() {
    if !ninja_available() {
        eprintln!("SKIP: ninja unavailable");
        return;
    }
    let dir = tempdir().expect("temp dir");
    let build = dir.path().join("build");
    fake_build_tree(&build, true);

    let f = fixture_with_selector(
        &format!(
            "mutation_ctest --test-dir {} -R ^NoSuchTestIsRegistered$",
            build.display()
        ),
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the arithmetic is wrong\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );

    assert_rejected(&f, "matches no registered test");
}

#[test]
fn a_sound_case_passes_and_leaves_its_subject_where_it_found_it() {
    let body = "fn keep(x: u8) -> u8 {\n    x + 1\n}\n";
    let f = fixture(
        body,
        "mutation_case \"the arithmetic is wrong\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );

    let (ok, output) = check(&f.casefile);
    assert!(ok, "the check mode refused a sound casefile:\n{output}");
    assert!(
        output.contains("1/1 case(s) still apply"),
        "a sound casefile did not report as applying:\n{output}"
    );
    // The mode applies each case for real — that is what makes it evidence
    // rather than a parse — so the restore is half of what it promises.
    assert_eq!(
        fs::read_to_string(&f.target).expect("read the subject back"),
        body,
        "the check mode left its subject mutated"
    );
}

/// The harness's artifact hash, for a file on disk.
///
/// Sourced the way `scripts/mutate` sources it, so what these tests measure
/// is the function the harness actually calls.
fn artifact_hash(path: &Path) -> String {
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "MUTATION_OBJCOPY=\"$(command -v objcopy)\"; \
             source scripts/lib/mutation_artifact_hash.sh; \
             mutation_artifact_hash {}",
            path.display()
        ))
        .current_dir(repo_root())
        .output()
        .expect("run the artifact hash");
    assert!(
        out.status.success(),
        "hashing {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// This test binary, which is an ELF carrying debug info and a build-id.
fn own_executable() -> PathBuf {
    std::env::current_exe().expect("a test binary has a path")
}

fn objcopy(args: &[&str]) -> bool {
    Command::new("objcopy")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn the_artifact_hash_ignores_the_build_id_note() {
    // Measured on this repository's C++ build: two builds of one unchanged
    // source differ in the DWARF 5 skeleton's `DWO ID` — eight bytes the
    // compiler picks anew per invocation — and in `.note.gnu.build-id`,
    // which is a hash of the content and therefore follows. A harness that
    // hashed the whole file read that as "the restore did not reproduce the
    // baseline", which is a verdict about a mutation drawn from a nonce.
    let dir = tempdir().expect("temp dir");
    let stripped = dir.path().join("no-build-id");
    if !objcopy(&[
        "--remove-section=.note.gnu.build-id",
        own_executable().to_str().expect("utf-8 path"),
        stripped.to_str().expect("utf-8 path"),
    ]) {
        eprintln!("SKIP: objcopy unavailable");
        return;
    }
    assert_eq!(
        artifact_hash(&own_executable()),
        artifact_hash(&stripped),
        "two ELFs that differ only in their build-id note must hash alike"
    );
}

#[test]
fn the_artifact_hash_ignores_the_debug_sections() {
    let dir = tempdir().expect("temp dir");
    let stripped = dir.path().join("no-debug");
    if !objcopy(&[
        "--strip-debug",
        own_executable().to_str().expect("utf-8 path"),
        stripped.to_str().expect("utf-8 path"),
    ]) {
        eprintln!("SKIP: objcopy unavailable");
        return;
    }
    assert_eq!(
        artifact_hash(&own_executable()),
        artifact_hash(&stripped),
        "debugging sections must not reach the hash a verdict is drawn from"
    );
}

#[test]
fn the_artifact_hash_still_separates_two_different_programs() {
    // The direction that matters more: normalisation must not flatten the
    // question. Two different executables have to hash differently, or
    // "the mutation reached the binary" would be answerable by nothing.
    let dir = tempdir().expect("temp dir");
    let altered = dir.path().join("altered");
    // `--add-section` changes allocated content rather than debug metadata,
    // so the normalised hash is required to notice it.
    let payload = dir.path().join("payload.bin");
    fs::write(&payload, b"a section the original does not carry").expect("write payload");
    if !objcopy(&[
        "--add-section",
        &format!(".sce_probe={}", payload.display()),
        own_executable().to_str().expect("utf-8 path"),
        altered.to_str().expect("utf-8 path"),
    ]) {
        eprintln!("SKIP: objcopy unavailable");
        return;
    }
    assert_ne!(
        artifact_hash(&own_executable()),
        artifact_hash(&altered),
        "a binary carrying content the other does not must hash differently"
    );
}

/// Feed captured runner output to one of the failure-name parsers.
fn failure_names(function: &str, captured: &str) -> Vec<String> {
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
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn a_gtest_failure_is_named_through_ctests_job_prefix() {
    // `ctest -j` puts `N: ` in front of every line a test prints, so the
    // marker is not at the start of the line. gtest also prints each failure
    // twice — once where it happens, once in a summary that opens with a
    // COUNT where a test name would be.
    let names = failure_names(
        "mutation_failures_from_gtest",
        "1: [ RUN      ] ResumeSuite.ReturnsSomewhereTheDefaultDoesNot\n\
         1: [  FAILED  ] ResumeSuite.ReturnsSomewhereTheDefaultDoesNot (2 ms)\n\
         2: [       OK ] OtherSuite.StillGreen (1 ms)\n\
         1: [  FAILED  ] 1 test, listed below:\n\
         1: [  FAILED  ] ResumeSuite.ReturnsSomewhereTheDefaultDoesNot\n",
    );
    assert_eq!(
        names,
        vec!["ResumeSuite.ReturnsSomewhereTheDefaultDoesNot".to_string()],
        "the gtest parser must name each red test once, and must not read \
         the summary line's count as a test called `1`"
    );
}

#[test]
fn a_cargo_failure_is_named_once_and_not_again_from_the_index() {
    // libtest prints the name twice as well: the verdict line, then a
    // `failures:` index. Counting both would double whatever a caller
    // derives from this.
    let names = failure_names(
        "mutation_failures_from_cargo",
        "running 3 tests\n\
         test a_thing_that_passes ... ok\n\
         test inner::a_thing_that_does_not ... FAILED\n\
         test another_pass ... ok\n\n\
         failures:\n    inner::a_thing_that_does_not\n\n\
         test result: FAILED. 2 passed; 1 failed; 0 ignored\n",
    );
    assert_eq!(
        names,
        vec!["inner::a_thing_that_does_not".to_string()],
        "the cargo parser must name the red test exactly once"
    );
}

#[test]
fn a_green_run_names_nothing() {
    // The direction that matters for a mutation round: a parser that
    // hallucinated a name on a passing run would make every SURVIVED
    // verdict look attributable.
    assert!(failure_names(
        "mutation_failures_from_gtest",
        "1: [       OK ] OtherSuite.StillGreen (1 ms)\n1: [  PASSED  ] 1 test.\n",
    )
    .is_empty());
    assert!(failure_names(
        "mutation_failures_from_cargo",
        "running 1 test\ntest a_thing ... ok\n\n\
         test result: ok. 1 passed; 0 failed; 0 ignored\n",
    )
    .is_empty());
}

#[test]
fn a_library_selector_resolves_against_the_crate_type_cargo_reports() {
    // `--lib` in `LIVE_SELECTOR` is the point of this test: cargo reports
    // sce-build's library target as `rlib`, and a checker matching the
    // literal string "lib" called every `--lib` casefile broken. Two in this
    // corpus use that selector, so the defect read as a dead corpus rather
    // than as a bug in the check.
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"a case behind a --lib selector\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );
    let (ok, output) = check(&f.casefile);
    assert!(
        ok,
        "a `--lib` selector was rejected against a workspace whose library \
         target exists:\n{output}"
    );
}
