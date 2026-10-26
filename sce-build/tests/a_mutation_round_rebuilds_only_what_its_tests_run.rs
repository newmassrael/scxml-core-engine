// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A ctest round rebuilds what its tests run, not the whole tree.
//
// `scripts/mutate` used to rebuild the default target for every case. On this
// repository's C++ tree that is every generated machine: a template mutation
// regenerates all of them, and measured on run 39948952108 (2026-10-23) one case
// cost 90 to 125 minutes to run one test executable, so three of six
// static-datamodel casefiles reached the lane's 330-minute ceiling with a case
// still unjudged. The harness now builds the closure of the artifacts the
// selected tests run, and leaves out of the run the setup fixture that would
// build the rest anyway (`sce_built` in `tests/CMakeLists.txt`).
//
// What this measures is the observable difference, not the cost. The tree below
// has a second executable no test runs, and the mutant breaks only that one:
// the whole-tree build fails on it, so the old harness read the case as a tree
// that does not compile, while the closure never builds it and the oracle turns
// red. The tree also carries a whole-tree build fixture, as this repository's
// does, which fails on the same target: a harness that built the closure but
// left the fixture in the run would stop before the oracle ran.
//
// The control is the same tree under a generator whose graph the harness does
// not read. There it must still build the whole tree, and the case is
// INCONCLUSIVE: the fallback is what keeps a build closure the harness cannot
// name from being guessed.
//
// What a test runs is not only what its command names. A test handed an
// executable through its `ENVIRONMENT` spawns it, and that executable is part
// of what the round must build and hash, or a mutation that reaches only it is
// read as reaching nothing.
//
// The real `scripts/mutate` is run, over a real CMake/CTest project, for the
// reason `mutation_round_survives_a_moved_baseline` gives: the code under test is
// the harness's own build step, and a copy of it here would be a second spelling.

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

/// A CMake/CTest project inside the repository, under a gitignored path, for
/// the reason `mutation_round_survives_a_moved_baseline` states: the ctest
/// runner refuses an artifact whose real path is outside the repository.
struct Project {
    root: PathBuf,
    casefile: PathBuf,
    path: String,
    _ledger: TempDir,
    ledger: PathBuf,
    _home: TempDir,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const SUBJECT: &str = r#"int doubled(int x) {
    return x * 2;
}
"#;

const ORACLE: &str = r#"#include <stdio.h>

int doubled(int x);

int main(void) {
    if (doubled(3) != 6) {
        fprintf(stderr, "doubled\n");
        return 1;
    }
    return 0;
}
"#;

const BYSTANDER: &str = r#"int doubled(int x);

int main(void) {
    return doubled(0);
}
"#;

/// The program spawned by `Shape::Spawned`'s test: the oracle's check, in an
/// executable the test command does not name.
const HELPER: &str = ORACLE;

/// Runs the executable `G7_HELPER` names and passes its verdict on, as
/// `static_datamodel_lowered` runs the generator `SCE_CODEGEN_BIN` names.
const DRIVER: &str = r#"#include <stdlib.h>

int main(void) {
    const char *helper = getenv("G7_HELPER");
    if (helper == NULL) {
        return 2;
    }
    return system(helper) == 0 ? 0 : 1;
}
"#;

/// How the test `g7_oracle` reaches the code under test.
#[derive(Clone, Copy)]
enum Shape {
    /// It runs the executable that links the subject.
    Direct,
    /// It runs that executable through a script the configure step copies into
    /// the build tree: an artifact under the build directory that no edge of
    /// the graph produces, which is the shape the harness must not hand ninja.
    Wrapped,
    /// It runs a driver that links nothing under test and spawns, from its
    /// environment, the executable that does.
    Spawned,
}

/// Two executables over one source. Only the oracle is a test, and every test
/// requires a fixture whose setup builds the whole tree, as this repository's
/// tests require `sce_built`.
fn cmakelists(shape: Shape) -> String {
    let test = match shape {
        Shape::Direct => "add_test(NAME g7_oracle COMMAND g7_oracle)\n",
        Shape::Wrapped => {
            "configure_file(${CMAKE_CURRENT_SOURCE_DIR}/run.sh ${CMAKE_BINARY_DIR}/run.sh COPYONLY)\n\
             add_test(NAME g7_oracle COMMAND sh ${CMAKE_BINARY_DIR}/run.sh $<TARGET_FILE:g7_oracle>)\n"
        }
        Shape::Spawned => {
            "add_executable(g7_helper helper.c subject.c)\n\
             add_executable(g7_driver driver.c)\n\
             add_test(NAME g7_oracle COMMAND g7_driver)\n\
             set_tests_properties(g7_oracle PROPERTIES ENVIRONMENT \"G7_HELPER=$<TARGET_FILE:g7_helper>\")\n"
        }
    };
    format!(
        r#"cmake_minimum_required(VERSION 3.16)
project(g7_closure C)
enable_testing()
add_executable(g7_oracle oracle.c subject.c)
add_executable(g7_bystander bystander.c subject.c)
target_compile_definitions(g7_bystander PRIVATE G7_BYSTANDER=1)
add_test(NAME g7_tree_is_current
    COMMAND ${{CMAKE_COMMAND}} --build ${{CMAKE_BINARY_DIR}})
set_tests_properties(g7_tree_is_current PROPERTIES FIXTURES_SETUP g7_built)
{test}set_tests_properties(g7_oracle PROPERTIES FIXTURES_REQUIRED g7_built TIMEOUT 30)
"#
    )
}

const RUN: &str = "exec \"$@\"\n";

const LABEL: &str = "doubling is not adding";

/// The mutant is wrong for the oracle and does not compile for the bystander.
/// It stands for a template mutation that breaks one generated machine while
/// the machine a casefile's test runs still builds.
const CASES: &str = r##"
mutation_case "doubling is not adding" <<'PY'
edit(TARGET, "    return x * 2;", "#ifdef G7_BYSTANDER\n#error a target no selected test runs\n#endif\n    return x + 2;")
PY
"##;

fn run(cmd: &mut Command, what: &str) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("run {what}: {e}"));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{what} failed:\n{text}");
    text
}

/// `generator` is passed to CMake as is; `ninja` is put on the round's `PATH`
/// either way, so a control differs from the subject in what it names alone.
fn project(name: &str, generator: &str, shape: Shape, ninja: &Path) -> Project {
    let root = repo_root().join("tmp").join(format!("g7-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the fixture directory");

    fs::write(root.join("subject.c"), SUBJECT).expect("write subject.c");
    fs::write(root.join("oracle.c"), ORACLE).expect("write oracle.c");
    fs::write(root.join("bystander.c"), BYSTANDER).expect("write bystander.c");
    fs::write(root.join("run.sh"), RUN).expect("write run.sh");
    fs::write(root.join("helper.c"), HELPER).expect("write helper.c");
    fs::write(root.join("driver.c"), DRIVER).expect("write driver.c");
    fs::write(root.join("CMakeLists.txt"), cmakelists(shape)).expect("write CMakeLists.txt");

    let ninja_dir = ninja.parent().expect("ninja lives in a directory");
    let path = match std::env::var_os("PATH") {
        Some(rest) => {
            let mut dirs = vec![ninja_dir.to_path_buf()];
            dirs.extend(std::env::split_paths(&rest));
            std::env::join_paths(dirs)
                .expect("join PATH")
                .to_string_lossy()
                .into_owned()
        }
        None => ninja_dir.to_string_lossy().into_owned(),
    };

    let build = root.join("build");
    run(
        Command::new("cmake")
            .env("PATH", &path)
            .arg("-S")
            .arg(&root)
            .arg("-B")
            .arg(&build)
            .arg("-G")
            .arg(generator)
            .arg("-DCMAKE_BUILD_TYPE=Release"),
        "cmake configure",
    );
    // A spawning tree is configured and not built, as the lane hands a round
    // its tree: what the test spawns must then be built by the round itself
    // before it can be found, let alone hashed.
    if !matches!(shape, Shape::Spawned) {
        run(
            Command::new("cmake")
                .env("PATH", &path)
                .arg("--build")
                .arg(&build),
            "cmake build",
        );
    }

    let rel = |p: &str| format!("tmp/g7-{name}/{p}");
    let casefile = root.join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "mutation_ctest --test-dir {build} -R '^g7_oracle$'\n\
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
        path,
        ledger: ledger.path().to_path_buf(),
        _ledger: ledger,
        _home: tempdir().expect("temp home"),
    }
}

impl Project {
    fn round(&self) -> String {
        let out = Command::new(repo_root().join("scripts/mutate"))
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("PATH", &self.path)
            .env("HOME", self._home.path())
            .env("SCE_MUTATION_LEDGER_DIR", &self.ledger)
            .env("SCE_BUILD_JOBS", "2")
            .output()
            .expect("run scripts/mutate");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        text
    }
}

fn ninja() -> Option<PathBuf> {
    sce_build::toolchain::require_or_skip("ninja", "the build closure a ctest round reads")
}

#[test]
fn a_case_is_judged_by_its_tests_when_a_target_they_do_not_run_breaks() {
    let Some(ninja) = ninja() else { return };
    let p = project("closure", "Ninja", Shape::Direct, &ninja);
    let output = p.round();

    assert!(
        output.contains(&format!("CAUGHT        {LABEL}")),
        "the oracle did not judge the case; a target no test runs decided it:\n{output}"
    );
    assert!(
        output.contains("each case rebuilds 1 build output(s) and their closure"),
        "the round did not say it builds the closure of what it runs:\n{output}"
    );
    // The fixture, by name, because a closure build with the fixture still in
    // the run fails it on the bystander and reads that failure as the red.
    assert!(
        output.contains("the whole-tree build fixture 'g7_built' is left out of the run"),
        "the whole-tree build fixture was left in the run:\n{output}"
    );
    assert!(
        !output.contains("g7_tree_is_current"),
        "the whole-tree build ran as a test of the round:\n{output}"
    );
}

#[test]
fn a_tree_whose_graph_the_harness_does_not_read_is_built_whole() {
    let Some(ninja) = ninja() else { return };
    let p = project("whole-tree", "Unix Makefiles", Shape::Direct, &ninja);
    let output = p.round();

    assert!(
        output.contains("each case rebuilds the whole tree"),
        "the round did not say it falls back to the whole tree:\n{output}"
    );
    assert!(
        output.contains(&format!("INCONCLUSIVE  {LABEL}")),
        "a whole-tree build of a mutant the bystander refuses was not INCONCLUSIVE:\n{output}"
    );
}

#[test]
fn an_artifact_no_edge_builds_is_not_handed_to_ninja() {
    let Some(ninja) = ninja() else { return };
    let p = project("wrapped", "Ninja", Shape::Wrapped, &ninja);
    let output = p.round();

    // A script may run any binary, so the closure of what it runs cannot be
    // read off the graph: the round names the artifact that stopped it and
    // builds the whole tree, where the bystander refuses the mutant.
    assert!(
        output.contains("each case rebuilds the whole tree:")
            && output.contains("/run.sh is not a build output"),
        "the round did not fall back on an artifact no edge builds:\n{output}"
    );
    assert!(
        output.contains(&format!("INCONCLUSIVE  {LABEL}")),
        "a whole-tree build of a mutant the bystander refuses was not INCONCLUSIVE:\n{output}"
    );
}

/// The test command names a driver the mutation never reaches; the executable
/// it spawns is the one the mutation changes. Read from the command alone, the
/// round neither rebuilds that executable nor sees it change, and calls the case
/// unreached: on run 40125467304 that was every case of the two casefiles that
/// hold `static_datamodel_lowered` to the generator it spawns.
#[test]
fn an_executable_a_test_is_handed_to_spawn_is_rebuilt_and_hashed() {
    let Some(ninja) = ninja() else { return };
    let p = project("spawned", "Ninja", Shape::Spawned, &ninja);
    let output = p.round();

    assert!(
        output.contains(&format!("CAUGHT        {LABEL}")),
        "the executable the test spawns did not judge the case:\n{output}"
    );
    assert!(
        output.contains("and 1 companion executable(s) the tests may spawn"),
        "the round did not resolve what the test is handed to spawn:\n{output}"
    );
    // The driver and the helper, and not the bystander: the helper is built
    // because the test spawns it, and nothing spawns the bystander.
    assert!(
        output.contains("each case rebuilds 2 build output(s) and their closure"),
        "the round did not build the executable the test spawns:\n{output}"
    );
}
