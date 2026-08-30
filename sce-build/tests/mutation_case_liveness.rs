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

/// Where a fixture's round keeps its in-flight record: beside the fixture,
/// never in the ambient store.
///
/// A round opens a record before it mutates and retires it after it restores,
/// and `mutation_inflight_recover` refuses to start when it finds an ABANDONED
/// record whose snapshot directory is gone — the right answer for a developer
/// tree, where that means a killed round left a mutation behind.
///
/// Without this, every test here shared one directory
/// (`mutation_ledger_root()/in-flight`) while cargo ran them concurrently, so
/// one test's finished round was another test's abandoned one: its pid had
/// exited and its `tempdir()` snapshot was already unlinked, which is exactly
/// the unrecoverable shape the guard hard-refuses. The refusal then stood
/// where the rejection under test should have been, and the assertion failed
/// reporting the wrong reason. Measured on job 98430990425 (sha bb02f7ad53):
/// 11 of 28 failed that way, every message naming a sibling's temp path, while
/// the same commit passed locally — the pass was ordering luck, not health.
///
/// Isolation also runs the other way: unscoped, these fixtures WRITE records
/// into the store a real round reads, so a crash here could refuse a
/// developer's next round over a mutation that was never in their tree.
/// `mutation_inflight.rs` already scopes its rounds this way.
fn inflight_dir(casefile: &Path) -> PathBuf {
    casefile
        .parent()
        .expect("a fixture casefile lives in its own directory")
        .join("in-flight")
}

/// Run the check mode over a fixture; return (success, combined output).
fn check(casefile: &Path) -> (bool, String) {
    let out = Command::new(repo_root().join("scripts/mutate"))
        .arg("--check")
        .arg(casefile)
        .env("SCE_MUTATION_INFLIGHT_DIR", inflight_dir(casefile))
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --check");

    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// Run the check mode over one shard of a fixture; return (success, output).
fn check_shard(casefile: &Path, shard: &str) -> (bool, String) {
    let out = Command::new(repo_root().join("scripts/mutate"))
        .arg("--check")
        .arg("--shard")
        .arg(shard)
        .arg(casefile)
        .env("SCE_MUTATION_INFLIGHT_DIR", inflight_dir(casefile))
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --check --shard");

    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// The case labels a run reported a verdict on, in the order it reported them.
///
/// Read off the console rather than counted, because the property under test
/// is WHICH cases ran and a count cannot tell "the first two" from "the last
/// two". `applies` is the check mode's verdict word; the run mode's are
/// CAUGHT, SURVIVED and INCONCLUSIVE.
fn labels_reported(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.trim_end().strip_prefix("applies "))
        .map(|label| label.trim().to_string())
        .collect()
}

/// A fixture whose cases are distinguishable by label and by anchor.
fn sharded_fixture(count: usize) -> (Fixture, Vec<String>) {
    let body: String = (0..count).map(|i| format!("line{i}\n")).collect();
    let cases: String = (0..count)
        .map(|i| {
            format!(
                "mutation_case \"case {i}\" <<'PY'\nedit(TARGET, \"line{i}\", \"LINE{i}\")\nPY\n\n"
            )
        })
        .collect();
    let labels = (0..count).map(|i| format!("case {i}")).collect();
    (fixture(&body, &cases), labels)
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

/// The N shards of a casefile partition its cases: each runs once, in
/// declaration order, and between them they run all of it.
///
/// This is what makes a casefile stop being the unit that has to fit inside a
/// CI job. `mutation-rounds.yml` used to give each casefile one job, and eight
/// of dispatch 32803356117's twenty-four rounds ran to the 330-minute ceiling
/// and were `cancelled` — an absent verdict, which reads as neither green nor
/// red. Splitting the file across jobs only helps if the split is exact:
/// overlap measures the same case twice and calls it coverage, and a gap
/// leaves cases nothing runs while every job reports success.
#[test]
fn the_shards_of_a_casefile_partition_its_cases_exactly() {
    let (f, declared) = sharded_fixture(5);

    let (ok, whole) = check(&f.casefile);
    assert!(ok, "the unsharded fixture did not pass:\n{whole}");
    assert_eq!(
        labels_reported(&whole),
        declared,
        "the fixture itself does not report its five cases in order, so nothing \
         below is measuring shards:\n{whole}"
    );

    // Three shards over five cases, so the remainder is exercised too: the
    // sizes must come out 2, 2, 1 rather than 1, 1, 3.
    let mut seen = Vec::new();
    for index in 1..=3 {
        let shard = format!("{index}/3");
        let (ok, output) = check_shard(&f.casefile, &shard);
        assert!(ok, "shard {shard} did not pass:\n{output}");
        let reported = labels_reported(&output);
        assert!(
            !reported.is_empty(),
            "shard {shard} reported a verdict on no case at all, and exited 0. \
             A round that measures nothing and passes is the failure this whole \
             harness is about:\n{output}"
        );
        assert!(
            output.contains(&format!("shard {shard}: case(s)")),
            "shard {shard} did not say which slice it was answering for, so its \
             console cannot be read afterwards for what it covered:\n{output}"
        );
        seen.extend(reported);
    }

    assert_eq!(
        seen, declared,
        "the three shards did not partition the five cases. Concatenated in \
         shard order they must be the casefile's own order, each case exactly \
         once — a repeat is the same case measured twice, and an omission is a \
         case no job runs while every job reports success."
    );
}

/// A shard the casefile cannot supply is refused, not run empty.
///
/// The lane derives N from the case count, so an out-of-range shard means the
/// selection and the casefile disagree. The dangerous answer is the quiet one:
/// a slice past the end has no cases, and a round that measures nothing exits
/// 0 and is recorded as a casefile that was judged.
#[test]
fn a_shard_the_casefile_cannot_supply_is_refused() {
    let (f, _) = sharded_fixture(2);

    let (ok, output) = check_shard(&f.casefile, "3/3");
    assert!(
        !ok,
        "a casefile of two cases accepted a third shard:\n{output}"
    );
    assert!(
        output.contains("would measure nothing"),
        "refused, but not for the reason under test:\n{output}"
    );

    for shard in ["0/2", "3/2", "2"] {
        let (ok, output) = check_shard(&f.casefile, shard);
        assert!(!ok, "`--shard {shard}` was accepted:\n{output}");
    }
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
fn a_quoted_summary_is_not_added_to_this_runs_own_tally() {
    // The counts come from lines libtest owns the START of. Before that
    // anchoring, the failed count was read with a pattern that matched
    // anywhere on any line — so a test that quoted ANOTHER run's summary had
    // that run's failures added to its own.
    //
    // Measured 2026-08-30: a suite whose probe re-executes its own binary and
    // quotes the child's output on failure made a round report `3/8 red` while
    // naming two tests. CAUGHT survived it because CAUGHT needs only a
    // non-zero count; a SURVIVED verdict read the same way would have been a
    // false one.
    let counts = failure_names(
        "mutation_counts_from_cargo",
        "running 8 tests\n\
         test a_pass ... ok\n\
         test the_probe ... FAILED\n\
         failures:\n\
         ---- the_probe stdout ----\n\
         the child half failed:\n\
         \x20   | running 1 test\n\
         \x20   | test the_probe ... FAILED\n\
         \x20   | test result: FAILED. 0 passed; 1 failed; 0 ignored\n\n\
         test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured\n",
    );
    assert_eq!(
        counts,
        vec!["8 1".to_string()],
        "the quoted child's header and summary must not reach this run's tally"
    );
}

#[test]
fn several_binaries_worth_of_tests_are_totalled() {
    // One captured stream can hold more than one `running N tests` header —
    // libtest prints one per test section — so the run count is a sum rather
    // than the last thing seen.
    let counts = failure_names(
        "mutation_counts_from_cargo",
        "running 2 tests\n\
         test result: ok. 2 passed; 0 failed; 0 ignored\n\
         running 3 tests\n\
         test result: FAILED. 1 passed; 2 failed; 0 ignored\n",
    );
    assert_eq!(
        counts,
        vec!["5 2".to_string()],
        "the tally must total every section the stream carried"
    );
}

#[test]
fn a_red_baseline_names_the_tests_it_is_made_of() {
    // The refusal above this list is a COUNT, and a count sends the reader
    // back to CI to ask which. Measured 2026-08-24: the first whole-corpus
    // sweep stopped `ci_lane_gate_selection.cases` with `baseline is not
    // green (2 failing)` and the job log carried nothing else to act on.
    let lines = failure_names(
        "mutation_baseline_failures",
        "the_push_hook_delegates_rather_than_carrying_gates\n\
         every_gate_script_is_executable\n",
    );
    assert_eq!(
        lines,
        vec![
            "the_push_hook_delegates_rather_than_carrying_gates".to_string(),
            "every_gate_script_is_executable".to_string(),
        ],
        "a red baseline must name every test it is made of"
    );
}

#[test]
fn a_wholesale_baseline_break_is_capped_and_says_how_many_it_hid() {
    // A baseline that broke wholesale can name hundreds, and the refusal is
    // what the reader must not lose off the top of the screen. Truncating
    // silently would be the same defect one size down, so the count of what
    // was hidden is part of the output.
    let many: String = (0..25)
        .map(|i| format!("some_test_{i}\n"))
        .collect::<Vec<_>>()
        .join("");
    let lines = failure_names("mutation_baseline_failures", &many);
    assert_eq!(
        lines.len(),
        21,
        "expected 20 names plus one overflow line; got {lines:?}"
    );
    assert_eq!(lines[0], "some_test_0");
    assert_eq!(
        lines[20], "(+5 more)",
        "the excerpt must say how many it did not print; got {:?}",
        lines[20]
    );
}

#[test]
fn a_baseline_the_parser_could_not_read_says_so_rather_than_nothing() {
    // The failure mode this whole file guards: a parser that stopped matching
    // the runner's format emits nothing, and nothing is indistinguishable
    // from the silence being repaired. "The run named none" and "there were
    // none" are different facts and the output has to separate them — the
    // caller only reaches this path when the count was non-zero.
    let lines = failure_names("mutation_baseline_failures", "");
    assert_eq!(lines.len(), 1, "expected exactly one line; got {lines:?}");
    assert!(
        lines[0].contains("named no failing test"),
        "an unreadable baseline must say so out loud; got {:?}",
        lines[0]
    );
}

/// Feed a captured build log to the refusal parser.
fn build_refusal(captured: &str) -> Vec<String> {
    let dir = tempdir().expect("temp dir");
    let sample = dir.path().join("build.log");
    fs::write(&sample, captured).expect("write the sample");

    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source scripts/lib/mutation_build_refusal.sh; \
             mutation_build_refusal < {}",
            sample.display()
        ))
        .current_dir(repo_root())
        .output()
        .expect("run the parser");
    assert!(
        out.status.success(),
        "mutation_build_refusal exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn a_refused_rust_build_is_quoted_from_the_compilers_first_line() {
    // The shape this parser exists for, taken verbatim from the round of
    // `f81cc8e500`: `warnings = "deny"` turns an unused binding into a hard
    // error, and the mutation that caused it is inexpressible as written.
    // `tail` would have returned the "could not compile" line, which names no
    // file, no line and no rule — the reason three cases sat INCONCLUSIVE for
    // a week with nothing to act on.
    let lines = build_refusal(
        "   Compiling thiserror v2.0.18\n\
         warning: unused import: `core::fmt`\n\
         error: unused variable: `reading`\n  \
           --> backends/rust/runtime/src/engine.rs:1356:61\n  \
           = help: if this is intentional, prefix it with an underscore\n\
         error: could not compile `sce-rust-runtime` (lib) due to 1 previous error\n",
    );
    assert_eq!(
        lines.first().map(String::as_str),
        Some("error: unused variable: `reading`"),
        "the excerpt must open on the compiler's FIRST diagnostic, not on the \
         summary line at the end and not on a preceding warning; got {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("engine.rs:1356")),
        "the excerpt must carry far enough to name the site; got {lines:?}"
    );
}

#[test]
fn a_refused_c_build_is_quoted_though_its_error_is_mid_line() {
    // The ctest runner's compilers write `path:line:col: error:` rather than
    // starting the line with the word, so a parser anchored to the start of a
    // line would report nothing for every C and C++ casefile in the corpus —
    // and reporting nothing is exactly the silence this replaced.
    let lines = build_refusal(
        "[42/91] Building CXX object tests/CMakeFiles/x.dir/Test.cpp.o\n\
         FAILED: tests/CMakeFiles/x.dir/Test.cpp.o\n\
         /home/coin/scxml-core-engine/sce/src/Engine.cpp:88:5: error: \
         'noteReading' was not declared in this scope\n\
             88 |     noteReading(event);\n\
         ninja: build stopped: subcommand failed.\n",
    );
    assert!(
        lines
            .first()
            .is_some_and(|l| l.contains("Engine.cpp:88:5: error:")),
        "the excerpt must open on the compiler's diagnostic even when the \
         word `error` is not at the start of the line; got {lines:?}"
    );
}

#[test]
fn a_build_that_failed_without_a_diagnostic_still_says_something() {
    // A build can refuse for a reason no compiler printed — a missing
    // toolchain, a full disk, a linker killed by the OOM killer — and those
    // arrive at the END. An excerpt that went empty whenever the parser did
    // not recognise the failure would be indistinguishable from the silence
    // this file's subject replaced, so the tail is the fallback.
    let lines = build_refusal(
        "   Compiling sce-rust-runtime v0.1.0\n\
         rustc: /usr/bin/ld: final link failed: No space left on device\n\
         collect2: fatal, exiting\n",
    );
    assert!(
        lines.iter().any(|l| l.contains("No space left on device")),
        "a failure with no `error:` line must still be quoted; got {lines:?}"
    );
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
