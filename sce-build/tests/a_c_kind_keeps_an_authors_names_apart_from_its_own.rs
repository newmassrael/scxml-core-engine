// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated C11 does (see `common::name_oracle` for
//! the question every backend is asked, and `common::native_oracle` for the
//! compile loop C shares with C++).
//!
//! C has one namespace for functions, variables and typedef names in a scope,
//! and the headers the generated code includes bring in names of all three
//! (`size_t`, `memcpy`, `uint8_t`) and macros besides (`NULL`, `true`). A
//! parameter called `size_t` beside a use of `size_t`, or called like a
//! function the body calls, is a compile error where another language would
//! have kept the two apart.
//!
//! Every document of the eight kinds is generated with each name it declares
//! renamed to each candidate and each result is compiled as a translation unit
//! (`-std=c11 -Wall -Wextra -Werror -fsyntax-only`, the contract the generated
//! headers are held to). A name the generator refuses is an answer. What may
//! not happen is a name that is accepted and then makes the generated C not
//! compile.
//!
//! The generated C is compiled, not run: C is statically typed, so a name that
//! hides a type or a function the body uses is a compile error, which is what
//! this catches. A name that is accepted, compiles and does something else is
//! not seen here.
//!
//! C cannot bring a function in under another name, so an author's name that is
//! the function of a document it imports hides it: those candidates are counted
//! and not asked (`imported_function`), and said in the totals.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::name_oracle::names_by_kind;
use common::native_oracle::{run, Native};
use common::source_lexing::Lang;
use regex::Regex;
use sce_build::generator::Language;

/// The names the headers the generated C includes (`<stdint.h>`, `<stdbool.h>`,
/// `<stddef.h>`, `<string.h>`, `<stdio.h>`) and the math and library functions
/// an author meets, which the language and the templates fix.
const UNIVERSE: &[&str] = &[
    "size_t",
    "ptrdiff_t",
    "wchar_t",
    "int8_t",
    "int16_t",
    "int32_t",
    "int64_t",
    "uint8_t",
    "uint16_t",
    "uint32_t",
    "uint64_t",
    "intptr_t",
    "uintptr_t",
    "intmax_t",
    "uintmax_t",
    "bool",
    "true",
    "false",
    "null",
    "errno",
    "stdin",
    "stdout",
    "stderr",
    "memcpy",
    "memset",
    "memcmp",
    "memmove",
    "strlen",
    "strcmp",
    "strncmp",
    "strcpy",
    "strncpy",
    "strcat",
    "printf",
    "fprintf",
    "sprintf",
    "snprintf",
    "puts",
    "putchar",
    "abs",
    "labs",
    "fabs",
    "fabsf",
    "sqrt",
    "floor",
    "ceil",
    "round",
    "trunc",
    "pow",
    "fmod",
    "isnan",
    "isinf",
    "isfinite",
    "signbit",
    "assert",
    "free",
    "malloc",
    "calloc",
    "realloc",
    "exit",
    "abort",
    "min",
    "max",
    "offsetof",
    "sizeof",
    "va_list",
    "main",
];

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/expected")
}

/// The runtime headers a generated header includes: the forge runtime's
/// (`sce/forge/…`) and the platform one's (`sce/portability.h`).
fn c_runtime_includes() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/c");
    vec![
        root.join("forge-runtime/include"),
        root.join("runtime/include"),
    ]
}

/// What C asks of the shared oracle.
fn c() -> Native {
    Native {
        label: "c",
        language: Language::C11,
        resource_dir: resource_dir(),
        expected_dir: expected_dir(),
        output_extensions: &["c.h"],
        excluded_suffixes: &[],
        lang: Lang::CFamily,
        universe: UNIVERSE,
        probe_extension: "c",
        compilers: &["clang", "gcc", "cc"],
        compile_flags: &["-std=c11", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"],
        include_dirs: c_runtime_includes(),
        imported_function: Some(r"(?m)^static inline [^;{(]*?\b([A-Za-z_][A-Za-z0-9_]*)\("),
    }
}

#[test]
fn an_authors_name_never_decides_whether_the_generated_c_of_a_kind_builds() {
    run(&c());
}

/// C's own keywords, which no author can name a local (the generator refuses
/// them in the document) and so are not the list's business.
const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
    "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
    "union", "unsigned", "void", "volatile", "while",
];

/// Every name a library header the committed C includes declares, and the
/// generated C uses, is one the generator keeps an author's local off.
///
/// The headers are read as the compiler reads them: the committed C's own
/// `#include` lines are preprocessed (`cc -E -dD`) and every macro, typedef and
/// function the result declares is a name. What the committed output uses of
/// those, in lowercase (a local is spelled snake_case, so it can only meet a
/// lowercase name) and not under a prefix the spelling already shifts, must come
/// back different from `c11_local_spelling`. A template that begins to call one
/// more library function fails here with its name.
#[test]
fn every_library_name_the_generated_c_uses_is_one_the_generator_escapes() {
    let Some(cc) = sce_build::toolchain::require_any_or_skip(
        &["clang", "gcc", "cc"],
        "the C library name pin",
    ) else {
        return;
    };
    let from_outputs = names_by_kind(&expected_dir(), &["c.h"], &[], Lang::CFamily);
    // The library headers the committed C includes, as the include lines name them.
    let include = Regex::new(r#"(?m)^#include ([<"][^>"]+[>"])"#).expect("regex");
    let mut headers: BTreeSet<String> = BTreeSet::new();
    for entry in std::fs::read_dir(expected_dir()).expect("read the committed outputs") {
        let path = entry.expect("directory entry").path();
        if !path.to_string_lossy().ends_with(".c.h") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("a committed C output");
        for caps in include.captures_iter(&source) {
            let spec = caps[1].to_string();
            // A sibling document's header (`"condition_threshold.h"`) is not a library's.
            if spec.starts_with('<') || spec.contains('/') {
                headers.insert(spec);
            }
        }
    }
    assert!(
        headers.len() >= 4,
        "implausibly few library headers ({headers:?}); the scan broke"
    );
    let dir = std::env::temp_dir().join(format!("sce_c_library_names_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    let probe: String = headers.iter().map(|h| format!("#include {h}\n")).collect();
    std::fs::write(dir.join("probe.c"), probe).expect("write the probe");
    let mut command = Command::new(&cc);
    command.args(["-E", "-dD", "-std=c11"]);
    for include in c_runtime_includes() {
        command.arg("-I").arg(include);
    }
    let out = command
        .arg("probe.c")
        .current_dir(&dir)
        .output()
        .expect("the C compiler preprocesses");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "preprocessing the library headers failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);

    // A typedef is read only where it is named the library's way (`size_t`): the
    // first `;` after `typedef` ends a struct's FIRST MEMBER, and a member
    // (`data`, `failed`) is in a namespace of its own, which no local meets.
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for pattern in [
        r"(?m)^#define ([A-Za-z_][A-Za-z0-9_]*)",
        r"typedef[^;]*?\b([A-Za-z_][A-Za-z0-9_]*_t)\s*;",
        r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(",
    ] {
        let found = Regex::new(pattern).expect("regex");
        declared.extend(found.captures_iter(&text).map(|c| c[1].to_string()));
    }

    let needed: BTreeSet<String> = from_outputs
        .values()
        .flatten()
        .filter(|name| declared.contains(*name))
        .filter(|name| **name == name.to_lowercase())
        .filter(|name| !C_KEYWORDS.contains(&name.as_str()))
        .filter(|name| !name.starts_with('_') && !name.starts_with("sce_"))
        .cloned()
        .collect();
    assert!(
        needed.len() >= 5,
        "implausibly few library names are used ({needed:?}); the derivation broke"
    );
    let unescaped: Vec<&String> = needed
        .iter()
        .filter(|name| sce_build::forge::generator::c11_local_spelling(name) == **name)
        .collect();
    assert!(
        unescaped.is_empty(),
        "the generated C uses library names the generator does not escape: {unescaped:?}"
    );
}
