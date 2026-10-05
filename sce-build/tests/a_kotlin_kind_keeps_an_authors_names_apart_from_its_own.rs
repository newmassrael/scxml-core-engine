// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated Kotlin does (see `common::name_oracle` for
//! the question every backend is asked, and `common::native_oracle` for the
//! compile loop Kotlin shares with C, C++ and Rust).
//!
//! Kotlin writes an author's name as written, and reads the members of its own
//! class bare: an observer's monitors are properties of the class and a filter
//! holds its smoother in one, so a parameter of the same name that the method
//! reads first hides the property. A function the file brings in with `import`
//! (`lookup`, `linear`, `bilinear`) and a sibling document's package, brought in
//! whole with `import com.sce.generated.<document>.*`, are names a local can
//! meet too. Which of them a local of the same name hides is the compiler's
//! answer and not a guess, which is what this oracle asks it.
//!
//! Every document of the eight kinds is generated with each name it declares
//! renamed to each candidate and each result is compiled (`-Werror`, the
//! contract the generated files are held to) against the forge runtime, built
//! once from this tree. `kotlinc` is a JVM that takes seconds to start, so it is
//! run on hundreds of renamings at a time and what it says is told back to the
//! renaming whose file it names. Each renaming is its own package, so none can
//! see another.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::native_oracle::{run, Batch, Native};
use common::source_lexing::Lang;
use sce_build::generator::Language;

/// The names the files the generated Kotlin includes bring into scope, and the
/// library names an author meets, which the language and the templates fix.
const UNIVERSE: &[&str] = &[
    "it",
    "this",
    "abs",
    "min",
    "max",
    "println",
    "lookup",
    "linear",
    "bilinear",
    "events",
    "delta",
    "impl",
    "value",
    "result",
    "valid",
    "reason",
    "size",
    "length",
    "run",
    "let",
    "also",
    "apply",
    "kotlin",
    "com",
    "sce",
    "main",
    "toInt",
    "toLong",
    "toDouble",
    "ValidationResult",
    "EventQueue",
    "ThresholdState",
    "ForgeDomainTag",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The file Kotlin's generator writes a document's unit to: the document's name
/// in PascalCase.
fn file_name(stem: &str) -> String {
    format!(
        "{}.kt",
        sce_build::filters::to_pascal_case(stem.to_string())
    )
}

/// Every Kotlin source under `dir`.
fn kotlin_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read the runtime sources") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            kotlin_sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "kt") {
            found.push(path);
        }
    }
    found.sort();
}

/// Build the forge runtime a generated file imports (`com.sce.forge.runtime`)
/// into a jar, once. Its common sources import nothing but the standard library,
/// so the compiler needs nothing else.
fn runtime_jar(kotlinc: &Path, scratch: &Path) -> PathBuf {
    let mut sources = Vec::new();
    kotlin_sources(
        &repo_root().join("backends/kotlin/forge-runtime/src/commonMain"),
        &mut sources,
    );
    assert!(
        sources.len() >= 8,
        "implausibly few runtime sources ({sources:?}); the scan broke"
    );
    let jar = scratch.join("forge-runtime.jar");
    let out = Command::new(kotlinc)
        .arg("-d")
        .arg(&jar)
        .args(&sources)
        .output()
        .expect("kotlinc runs");
    assert!(
        out.status.success() && jar.is_file(),
        "the forge runtime does not build:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    jar
}

#[test]
fn an_authors_name_never_decides_whether_the_generated_kotlin_of_a_kind_builds() {
    let Some(kotlinc) =
        sce_build::toolchain::require_any_or_skip(&["kotlinc"], "the Kotlin kind name oracle")
    else {
        return;
    };
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let jar = runtime_jar(&kotlinc, scratch.path());
    run(&Native {
        label: "kotlin",
        language: Language::Kotlin,
        resource_dir: repo_root().join("tests/forge/resources"),
        expected_dir: repo_root().join("tests/forge/expected"),
        output_extensions: &["kt"],
        excluded_suffixes: &[],
        lang: Lang::Kotlin,
        universe: UNIVERSE,
        // No probe: the compiler is run on the units themselves.
        probe_extension: "kt",
        compilers: &["kotlinc"],
        compile_flags: &["-Werror"],
        include_dirs: Vec::new(),
        include_flag: None,
        extra_flags: vec!["-cp".to_string(), jar.display().to_string()],
        unit_suffix: ".kt",
        sibling_pattern: r"(?m)^import com\.sce\.generated\.([a-z0-9_]+)\.\*",
        file_name: Some(file_name),
        probe_line: |_| String::new(),
        batch: Some(Batch {
            per_run: 600,
            threads: 8,
            output_flag: "-d",
            diagnostic: r"(?m)^(\S+\.kt):\d+:\d+: (?:error|warning): (.*)$",
            env: &[("JAVA_OPTS", "-Xmx6g")],
        }),
        imported_function: None,
    });
}
