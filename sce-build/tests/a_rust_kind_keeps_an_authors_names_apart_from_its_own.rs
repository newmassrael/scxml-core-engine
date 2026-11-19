// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#[test]
fn an_authors_name_never_decides_whether_the_generated_rust_of_a_kind_builds() {
    if sce_build::toolchain::require_any_or_skip(&["rustc"], "the Rust kind name oracle").is_none()
    {
        return;
    }
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let flags = common::rust_runtime::rustc_flags(scratch.path());
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
