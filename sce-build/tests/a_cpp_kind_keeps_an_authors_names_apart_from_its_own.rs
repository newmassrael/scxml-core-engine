// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated C++ does (see `common::name_oracle` for
//! the question every backend is asked, and `common::native_oracle` for the
//! compile loop C++ shares with C).
//!
//! C++ writes an author's name as written, in the scope of the names the
//! generated header includes (`<cstdint>`, `<string>`, `<vector>`) and of the
//! types the body names. A parameter called like a type the body uses after it,
//! or like a function it calls, is a compile error. An imported function is
//! called through its namespace (`SCE::Generated::X::f`), which a local cannot
//! hide, so unlike C there is no name that is left out.
//!
//! Every document of the eight kinds is generated with each name it declares
//! renamed to each candidate and each result is compiled as a translation unit
//! (`-std=c++20 -Wall -Wextra -Werror -fsyntax-only`, the contract the generated
//! headers are held to; C++20 because a `bytes` parameter is a `std::span`).
//!
//! The candidates are every identifier the committed C++ writes, so a template
//! that begins to write one more library name is asked about it with no list to
//! extend. There is no companion test that reads the library headers for the
//! names a template writes bare, as the C oracle has: C has one namespace and
//! its header declarations are the names a local can hide, while a C++ header's
//! declarations are mostly members (`size`, `data`, `reset`) and namespace
//! members a local never meets, and a derivation that cannot tell those apart
//! needs a list of exceptions that would be the thing to go stale. The compiler
//! is asked instead, by this oracle, which names break the build.

mod common;

use std::path::{Path, PathBuf};

use common::native_oracle::{include_probe_line, run, Native, INCLUDE_SIBLING};
use common::source_lexing::Lang;
use sce_build::generator::Language;

/// The names the headers the generated C++ includes bring into scope, and the
/// library names an author meets, which the language and the templates fix.
const UNIVERSE: &[&str] = &[
    "size_t", "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t",
    "uint64_t", "bool", "true", "false", "null", "errno", "memcpy", "memset", "memcmp", "strlen",
    "strcmp", "abs", "fabs", "sqrt", "floor", "ceil", "round", "pow", "isnan", "isinf", "assert",
    "min", "max", "std", "string", "vector", "array", "optional", "tuple", "move", "swap", "begin",
    "end", "size", "data", "main",
];

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/expected")
}

#[test]
fn an_authors_name_never_decides_whether_the_generated_cpp_of_a_kind_builds() {
    run(&Native {
        label: "c++",
        language: Language::Cpp,
        resource_dir: resource_dir(),
        expected_dir: expected_dir(),
        // The committed C++ is a `.h`; the C backend's `.c.h` ends the same way.
        output_extensions: &["h"],
        excluded_suffixes: &[".c.h"],
        lang: Lang::CFamily,
        universe: UNIVERSE,
        probe_extension: "cpp",
        compilers: &["g++"],
        // C++20: a `bytes` parameter is a `std::span`.
        compile_flags: &["-std=c++20", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"],
        include_dirs: vec![
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/cpp/forge-runtime/include")
        ],
        include_flag: Some("-I"),
        extra_flags: Vec::new(),
        unit_suffix: ".h",
        sibling_pattern: INCLUDE_SIBLING,
        file_name: None,
        probe_line: include_probe_line,
        batch: None,
        imported_function: None,
    });
}
