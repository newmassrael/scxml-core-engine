// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A conformance suite emitted outside this repository builds and runs.
//!
//! `--output-dir` made the generated trees land wherever the caller
//! said, and that was taken to mean a vendoring repository could run
//! the W3C suite without SCE's build system. It could not. What landed
//! was a fragment: every generated Rust integration test named the
//! crate `sce_rust_tests`, every Go test imported
//! `github.com/newmassrael/sce-go-tests/harness`, every Kotlin source
//! sat in `com.sce.*`, and none of the files that make those trees a
//! package — manifest, module root, harness — was emitted at all.
//! Measured before the change: `cargo test` on the emitted tree failed
//! with `unresolved module or unlinked crate sce_rust_tests`.
//!
//! Two properties are asserted here, and only the second one is worth
//! much on its own:
//!
//!   1. The emitted sources name the suite the caller asked for, and
//!      the support files are there.
//!   2. The suite **builds and passes** under the language's own tool,
//!      from the emitted tree alone.
//!
//! The first without the second is how "vendoring works now" gets
//! claimed for a tree that has never been compiled — which is exactly
//! the state this test was written to end. Structural assertions that
//! a build would catch anyway are therefore kept to the ones that
//! localise a failure, not multiplied.
//!
//! Only the Rust half runs the build here, because `cargo` is the one
//! toolchain a Rust integration test can assume. The Go, Python and
//! Kotlin halves assert emission and leave the build to the per-language
//! gates that already own those toolchains
//! (`scripts/gates/w3c-{go,python,kotlin}.sh`), which is where a machine
//! without a JVM stops being this test's problem.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// One fixture: the claim is about packaging, and a package with one
/// module exercises every support file a package with two hundred does
/// while keeping the compile inside a test's budget. 144 is the
/// simplest — no script engine, no HTTP, no invoke — so a failure here
/// is about the suite rather than about the fixture.
const FIXTURE: &str = "144";

/// Emit a suite under `name` into a fresh directory and return the
/// output root. Panics with the generator's stderr on failure.
fn emit(language: &str, name: Option<&str>, out: &Path) {
    let root = repo_root();
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.arg("generate-w3c")
        .arg("-l")
        .arg(language)
        .arg("--registry")
        .arg(root.join(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH))
        .arg("--resources")
        .arg(root.join("resources"))
        .arg("--output-dir")
        .arg(out)
        .arg("-t")
        .arg(FIXTURE)
        // Pin the stamp so the emitted bytes cannot depend on the clock.
        .env("SOURCE_DATE_EPOCH", "0");
    if let Some(name) = name {
        cmd.arg("--suite-package").arg(name);
    }
    let output = cmd.output().expect("spawn sce-codegen generate-w3c");
    assert!(
        output.status.success(),
        "emitting a {language} suite must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The emitted Rust suite compiles and its fixture passes, under a
/// crate name this repository does not use.
///
/// The build is the assertion. Everything upstream of it — the
/// manifest, the crate root, the harness, the rewritten module path in
/// the integration test — has to be right simultaneously for `cargo
/// test` to reach a passing fixture, and any one of them being wrong
/// shows up here rather than in a consumer's tree.
#[test]
fn an_emitted_rust_suite_builds_and_passes_under_the_callers_own_crate_name() {
    let out = tempfile::tempdir().expect("tempdir");
    emit("rust", Some("acme-conformance"), out.path());

    let package = out.path().join("backends/rust/tests");
    let test_file = package.join(format!("tests/test_{FIXTURE}.rs"));
    let source = read(&test_file);
    assert!(
        source.contains("acme_conformance::generated::"),
        "the integration test must reach the suite by the caller's crate name; got:\n{source}",
    );
    assert!(
        !source.contains("sce_rust_tests"),
        "no reference to this repository's own crate may survive; got:\n{source}",
    );

    // Sharing the workspace target directory keeps this to the suite's
    // own compilation: the SCE runtime, Lua bindings and reqwest are
    // already built there by the time any test in this crate runs.
    let target_dir = repo_root().join("target/conformance-suite-standalone");
    let output = Command::new(env!("CARGO"))
        .arg("test")
        .current_dir(&package)
        .env("CARGO_TARGET_DIR", &target_dir)
        // Inherited RUSTFLAGS from the outer invocation would force a
        // full rebuild of every shared dependency under a different
        // fingerprint.
        .env_remove("RUSTFLAGS")
        .output()
        .expect("spawn cargo test in the emitted suite");
    assert!(
        output.status.success(),
        "the emitted suite must build and pass on its own.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("test test_{FIXTURE} ... ok")),
        "the fixture must actually run — a suite that compiles and \
         collects nothing passes for the wrong reason.\nstdout:\n{stdout}",
    );
}

/// The emitted Go module names the caller's module and carries what it
/// needs to resolve it.
#[test]
fn an_emitted_go_suite_names_the_callers_module() {
    let out = tempfile::tempdir().expect("tempdir");
    emit("go", Some("github.com/acme/conformance"), out.path());

    let module = out.path().join("backends/go/tests");
    let go_mod = read(&module.join("go.mod"));
    assert!(
        go_mod.contains("module github.com/acme/conformance"),
        "go.mod must declare the caller's module; got:\n{go_mod}",
    );
    assert!(
        go_mod.contains("=> ") && go_mod.contains("/backends/go/runtime"),
        "the SCE runtime must be redirected into the generating checkout; got:\n{go_mod}",
    );
    assert!(
        module.join("go.sum").is_file(),
        "the emitted module must carry the checksums for its one remote \
         dependency, or `go test` fails on a missing sum",
    );
    assert!(
        module.join("harness/harness.go").is_file(),
        "the harness every generated test imports must be emitted",
    );

    let test_file = module.join(format!("generated/test{FIXTURE}/test{FIXTURE}_test.go"));
    let source = read(&test_file);
    assert!(
        source.contains("\"github.com/acme/conformance/harness\""),
        "the generated test must import the harness from the caller's module; got:\n{source}",
    );
    assert!(
        !source.contains("sce-go-tests"),
        "no reference to this repository's own module may survive; got:\n{source}",
    );
}

/// The emitted Kotlin project sits in the caller's package root, and
/// keeps the SCE runtime packages where they are.
#[test]
fn an_emitted_kotlin_suite_moves_the_suite_packages_and_not_the_runtime() {
    let out = tempfile::tempdir().expect("tempdir");
    emit("kotlin", Some("com.acme.conformance"), out.path());

    let project = out.path().join("backends/kotlin/tests");
    for expected in [
        "settings.gradle.kts",
        "build.gradle.kts",
        "src/test/kotlin/com/acme/conformance/w3c/W3CTestBase.kt",
        "src/test/kotlin/com/acme/conformance/w3c/W3CHttpTestBase.kt",
        // The BasicHTTP test server one of those base classes drives.
        // Its absence is what the first emitted Kotlin suite failed to
        // compile on, and a list of packages could not have caught it.
        "src/main/kotlin/com/acme/conformance/http/W3CHttpTestServer.kt",
        "src/main/kotlin/com/acme/conformance/generated/test144/test144Sm.kt",
        "src/test/kotlin/com/acme/conformance/w3c/Test144.kt",
    ] {
        assert!(
            project.join(expected).is_file(),
            "an emitted Kotlin suite must carry {expected}",
        );
    }

    let settings = read(&project.join("settings.gradle.kts"));
    assert!(
        settings.contains("includeBuild("),
        "the SCE Kotlin projects are not published anywhere a consumer could \
         resolve them from, so the emitted settings must reach them as a \
         composite build; got:\n{settings}",
    );

    let machine = read(&project.join(format!(
        "src/main/kotlin/com/acme/conformance/generated/test{FIXTURE}/test{FIXTURE}Sm.kt"
    )));
    assert!(
        machine.contains("package com.acme.conformance.generated."),
        "the generated machine must sit in the caller's package; got:\n{machine}",
    );
    // The runtime is a dependency of the suite, not part of it: moving
    // its package would leave the emitted sources importing classes the
    // SCE artifacts do not contain.
    assert!(
        machine.contains("com.sce.runtime"),
        "the SCE runtime package must survive the rename; got:\n{machine}",
    );

    // Every emitted source declares the package its directory says it is
    // in.
    //
    // This is the assertion that does not move with the code. Which
    // packages belong to the suite is decided by one rule
    // (`KOTLIN_RUNTIME_PACKAGES`), and any check written in terms of
    // that rule agrees with it by construction — a mutation adding
    // `http` to the runtime list left `com.sce.http` in place and every
    // assertion above still passed, because the *path* comes from the
    // suite identity while only the *package clause* comes from the
    // rewrite. Requiring the two to agree is what catches a package the
    // rewrite forgot, and it is exactly what kotlinc would have said.
    let mut checked = 0;
    for (source_set, file) in kotlin_sources(&project) {
        let source = read(&file);
        let declared = source
            .lines()
            .find_map(|l| l.strip_prefix("package "))
            .unwrap_or_else(|| panic!("{} declares no package", file.display()))
            .trim();
        let expected_dir = source_set.join(declared.replace('.', "/"));
        assert_eq!(
            file.parent().expect("a file has a parent"),
            expected_dir,
            "{} declares `package {declared}`, which a Kotlin source tree \
             spells as a directory — a file whose path and package clause \
             disagree does not compile",
            file.display(),
        );
        checked += 1;
    }
    // Without a floor, a walk that found nothing would assert nothing
    // and print the same green as a clean one.
    assert!(
        checked >= 5,
        "only {checked} Kotlin source(s) were checked; the emitted suite \
         carries at least the three shipped classes, the machine and its \
         test",
    );
}

/// Every `.kt` under the emitted project, paired with the source-set
/// root it lives beneath — which is where a package's directory path is
/// measured from.
fn kotlin_sources(project: &Path) -> Vec<(PathBuf, PathBuf)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "kt") {
                out.push(path);
            }
        }
    }
    let mut found = Vec::new();
    for source_set in ["src/main/kotlin", "src/test/kotlin"] {
        let root = project.join(source_set);
        let mut files = Vec::new();
        walk(&root, &mut files);
        files.sort();
        found.extend(files.into_iter().map(|f| (root.clone(), f)));
    }
    found
}

/// The emitted Python tree carries the conftest its fixtures resolve
/// through, re-pointed at the generating checkout.
///
/// Python is the backend that names no suite — its wrappers import the
/// machine beside them by path — so what makes its tree standalone is
/// this one file and nothing else.
#[test]
fn an_emitted_python_suite_carries_a_conftest_that_reaches_the_runtime() {
    let out = tempfile::tempdir().expect("tempdir");
    emit("python", None, out.path());

    let tree = out.path().join("backends/python/tests");
    let conftest = read(&tree.join("conftest.py"));
    let runtime = repo_root().join("backends/python/runtime");
    assert!(
        conftest.contains(&runtime.display().to_string()),
        "the emitted conftest must reach the runtime in the generating \
         checkout, not a directory beside itself; got:\n{conftest}",
    );

    let wrapper = read(&tree.join(format!("generated/test{FIXTURE}/test_w3c_{FIXTURE}.py")));
    assert!(
        !wrapper.contains("parents[2]"),
        "the wrapper must not compute the runtime path from its own depth \
         below `backends/python/` — that directory does not exist in an \
         emitted suite, and the conftest already answers the question; \
         got:\n{wrapper}",
    );
}

/// Naming a suite while writing into this repository is refused.
///
/// The committed build files fix the name. Accepting a different one
/// would emit sources naming a package that does not exist, and the
/// failure would surface as a compile error in a tree the caller did
/// not think they were touching.
#[test]
fn renaming_the_suite_while_writing_into_this_repository_is_refused() {
    let root = repo_root();
    let output = Command::new(sce_codegen_bin())
        .arg("generate-w3c")
        .arg("-l")
        .arg("rust")
        .arg("--suite-package")
        .arg("acme-conformance")
        .arg("--list")
        .output()
        .expect("spawn sce-codegen generate-w3c");
    assert!(
        !output.status.success(),
        "a rename with no --output-dir must be refused, not silently applied",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--suite-package") && stderr.contains("--output-dir"),
        "the refusal must name both the flag that was wrong and the one that \
         makes it right; got: {stderr}",
    );
    // The repository is named so the reader knows which tree the run
    // was about to write into.
    assert!(
        stderr.contains(&root.display().to_string()),
        "the refusal must say which repository it means; got: {stderr}",
    );
}

/// The backends whose generated code names no suite refuse the flag
/// rather than accepting a name they will not use.
#[test]
fn the_backends_that_name_no_suite_refuse_the_flag() {
    let out = tempfile::tempdir().expect("tempdir");
    for (language, expected) in [("python", "conftest"), ("cpp", "CMake")] {
        let output = Command::new(sce_codegen_bin())
            .arg("generate-w3c")
            .arg("-l")
            .arg(language)
            .arg("--output-dir")
            .arg(out.path())
            .arg("--suite-package")
            .arg("whatever")
            .arg("--list")
            .output()
            .expect("spawn sce-codegen generate-w3c");
        assert!(
            !output.status.success(),
            "{language} emits nothing that names a suite, so --suite-package \
             must be refused rather than accepted and ignored",
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(expected),
            "the refusal must say why {language} has no suite name — expected \
             it to mention {expected:?}; got: {stderr}",
        );
    }
}

/// A name the target language could not spell is refused before any
/// file is written.
#[test]
fn an_unspellable_suite_name_is_refused_before_anything_is_written() {
    let out = tempfile::tempdir().expect("tempdir");
    let output = Command::new(sce_codegen_bin())
        .arg("generate-w3c")
        .arg("-l")
        .arg("rust")
        .arg("--output-dir")
        .arg(out.path())
        .arg("--suite-package")
        .arg("crate")
        .arg("-t")
        .arg(FIXTURE)
        .output()
        .expect("spawn sce-codegen generate-w3c");
    assert!(
        !output.status.success(),
        "`crate` is a Rust keyword, so no generated test could name it",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("keyword"),
        "the refusal must say what is wrong with the name; got: {stderr}",
    );
    let written: Vec<_> = std::fs::read_dir(out.path())
        .expect("read the output root")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        written.is_empty(),
        "a refused run must leave nothing behind; found {written:?}",
    );
}

/// The default emission is byte-identical to what the committed trees
/// carry, so an in-repo regeneration is unaffected by any of this.
///
/// Asserted against the committed file rather than against a second
/// run: comparing two runs of the same code proves only that the code
/// is deterministic, which was never in doubt. What matters is that
/// the suite-name substitution — which now runs on every emission — is
/// the identity at the default.
#[test]
fn the_default_emission_still_matches_the_committed_tree() {
    let out = tempfile::tempdir().expect("tempdir");
    emit("rust", None, out.path());

    let emitted_path = out
        .path()
        .join(format!("backends/rust/tests/tests/test_{FIXTURE}.rs"));
    // The committed tree is generator output *after* rustfmt — the
    // regeneration scripts run it — so the comparison runs the same
    // step rather than approximating it by ignoring whitespace.
    // Ignoring whitespace is not enough anyway: rustfmt also drops the
    // trailing comma it introduced, which a token-level comparison
    // reads as a real difference.
    let rustfmt = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .arg(&emitted_path)
        .output()
        .expect("spawn rustfmt");
    assert!(
        rustfmt.status.success(),
        "rustfmt must accept the emitted test; stderr: {}",
        String::from_utf8_lossy(&rustfmt.stderr),
    );

    let emitted = read(&emitted_path);
    let committed = read(&repo_root().join(format!("backends/rust/tests/tests/test_{FIXTURE}.rs")));
    assert_eq!(
        emitted, committed,
        "the default emission must still be the committed one — the \
         suite-name substitution has to be the identity at the default, or \
         every in-repo regeneration rewrites the whole tree. \
         (`SOURCE_DATE_EPOCH=0` pins the header stamp the committed file \
         also carries, so this is a byte comparison.)",
    );
    assert!(
        emitted.contains("sce_rust_tests::generated::"),
        "and it must still name this repository's own crate",
    );
}
