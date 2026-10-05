// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated Rust does (see `common::name_oracle` for
//! the question every backend is asked, and `common::native_oracle` for the
//! compile loop Rust shares with C and C++).
//!
//! Rust keeps a local, a type, a module and a function in namespaces of their
//! own, so most names that hide something in C do not here. What a local does
//! hide is a function the body calls bare (`lookup`, `linear`, which the
//! generated file brings in with `use`), a unit or tuple struct or variant that
//! a binding of that name would have to be a pattern for, and a name the
//! generator itself binds earlier in the same scope and the author's reads
//! after it. An imported document's function is called through its module
//! (`condition_threshold::check`), which a local cannot hide, so there is no
//! name that is left out.
//!
//! Every document of the eight kinds is generated with each name it declares
//! renamed to each candidate and each result is compiled as a module of a crate
//! (`--edition=2021 --crate-type=lib -D warnings`, the contract the generated
//! files are held to), against the two runtime crates a generated file names
//! (`sce_forge_runtime` and `sce_portable_bytes`), built once from this tree
//! with the lock file of the workspace so that the versions are the ones the
//! workspace tests. The crates are not compiled to code: `--emit=metadata`
//! type-checks and borrow-checks, which is what a name can break.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::native_oracle::{run, Native};
use common::source_lexing::Lang;
use sce_build::generator::Language;

/// The names a Rust file brings into scope, and the library names an author
/// meets, which the language, the prelude and the templates fix.
const UNIVERSE: &[&str] = &[
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "usize",
    "isize",
    "f32",
    "f64",
    "bool",
    "char",
    "str",
    "some",
    "none",
    "ok",
    "err",
    "vec",
    "string",
    "option",
    "result",
    "box",
    "abs",
    "min",
    "max",
    "std",
    "core",
    "alloc",
    "drop",
    "main",
    "lookup",
    "linear",
    "bilinear",
    "new",
    "default",
    "clone",
    "len",
    "events",
    "delta",
    "impl_",
    "heapless",
    "sce_forge_runtime",
    "sce_portable_bytes",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A crate root declares the module of every file the probe needs. `pub`, so a
/// function the file exports is reachable and not dead code.
fn probe_line(file: &str) -> String {
    format!("pub mod {};\n", file.trim_end_matches(".rs"))
}

/// Build the crates a generated file names, once, and answer the flags that make
/// `rustc` find them.
///
/// The crate that depends on them is a scratch one that is its own workspace and
/// carries a copy of the workspace's lock file, so cargo resolves the versions
/// the workspace resolved and, offline, from the cache the workspace filled.
/// What it built is read from cargo's own artifact messages, not guessed from a
/// file name, and the directory holds only this run's build, so there is one
/// candidate for every crate a flag has to find.
fn runtime_flags(scratch: &Path) -> Vec<String> {
    let root = repo_root();
    let crate_dir = scratch.join("runtime");
    std::fs::create_dir_all(crate_dir.join("src")).expect("mkdir");
    std::fs::write(crate_dir.join("src/lib.rs"), "").expect("write lib.rs");
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\n\
             name = \"sce-name-oracle-runtime\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             publish = false\n\
             \n\
             [lib]\n\
             path = \"src/lib.rs\"\n\
             \n\
             # The two crates the generated files name, with the heap tier the\n\
             # conformance crate builds them with.\n\
             [dependencies]\n\
             sce-forge-runtime = {{ path = {forge:?}, default-features = false, features = [\"alloc\"] }}\n\
             sce-portable-bytes = {{ path = {bytes:?}, default-features = false, features = [\"alloc\"] }}\n\
             \n\
             [workspace]\n",
            forge = root.join("backends/rust/forge-runtime").to_string_lossy(),
            bytes = root.join("backends/rust/portable-bytes").to_string_lossy(),
        ),
    )
    .expect("write Cargo.toml");
    std::fs::copy(root.join("Cargo.lock"), crate_dir.join("Cargo.lock"))
        .expect("copy the workspace lock file");

    let target = scratch.join("target");
    let manifest = crate_dir.join("Cargo.toml");
    let built = common::run_cargo_offline_first(|| {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--message-format=json")
            .arg("--manifest-path")
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", &target);
        cmd
    });
    assert!(
        built.output.status.success(),
        "the runtime crates a generated Rust file names do not build:\n{}",
        String::from_utf8_lossy(&built.output.stderr)
    );

    let stdout = String::from_utf8_lossy(&built.output.stdout);
    let mut flags = vec![
        "-L".to_string(),
        format!("dependency={}", target.join("debug/deps").display()),
    ];
    for crate_name in ["sce_forge_runtime", "sce_portable_bytes"] {
        let rlib = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|message| message["reason"] == "compiler-artifact")
            .filter(|message| message["target"]["name"] == crate_name)
            .flat_map(|message| message["filenames"].as_array().cloned().unwrap_or_default())
            .filter_map(|file| file.as_str().map(str::to_string))
            .find(|file| file.ends_with(".rlib"))
            .unwrap_or_else(|| panic!("cargo built no rlib of `{crate_name}`"));
        flags.push("--extern".to_string());
        flags.push(format!("{crate_name}={rlib}"));
    }
    flags
}

#[test]
fn an_authors_name_never_decides_whether_the_generated_rust_of_a_kind_builds() {
    if sce_build::toolchain::require_any_or_skip(&["rustc"], "the Rust kind name oracle").is_none()
    {
        return;
    }
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let flags = runtime_flags(scratch.path());
    run(&Native {
        label: "rust",
        language: Language::Rust,
        resource_dir: repo_root().join("tests/forge/resources"),
        expected_dir: repo_root().join("tests/forge/expected"),
        output_extensions: &["rs"],
        excluded_suffixes: &[],
        lang: Lang::Rust,
        universe: UNIVERSE,
        probe_extension: "rs",
        compilers: &["rustc"],
        compile_flags: &[
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "-D",
            "warnings",
        ],
        // rustc finds a sibling by the module the probe declares, not by a
        // directory it is told to search.
        include_dirs: Vec::new(),
        include_flag: None,
        extra_flags: flags,
        unit_suffix: ".rs",
        sibling_pattern: r"(?m)^use super::([A-Za-z0-9_]+)",
        file_name: None,
        probe_line,
        batch: None,
        imported_function: None,
    });
}
