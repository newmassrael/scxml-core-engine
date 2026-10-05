// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated C11 does (see
//! `common::name_oracle` for the question every backend is asked).
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

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use common::name_oracle::{
    candidates, documents, group, is_name_refusal, names_by_kind, rename, run_parallel, snake,
    with_name, Failure,
};
use common::source_lexing::Lang;
use regex::Regex;
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

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
fn c_runtime_includes() -> [PathBuf; 2] {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/c");
    [
        root.join("forge-runtime/include"),
        root.join("runtime/include"),
    ]
}

/// A generated C header: the file name it is written under and its source.
struct Header {
    file: String,
    source: String,
}

/// Generate `text` as C11 under the unique name `unique`: the headers, or the
/// reason it was refused.
fn generate(unique: &str, text: &str) -> Result<Vec<Header>, String> {
    compile_forge_with_imports(
        &with_name(text, unique),
        DocumentLabel::symmetric(unique),
        Language::C11,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| e.error.to_string())
    .map(|output| {
        output
            .files
            .into_iter()
            .filter(|(file, _)| file.ends_with(".h"))
            .map(|(file, source)| Header { file, source })
            .collect()
    })
}

/// The sibling headers a generated header includes by name (`#include
/// "condition_threshold.h"`), as opposed to the runtime's (`sce/forge/…`).
fn sibling_includes(source: &str) -> Vec<String> {
    let include = Regex::new(r#"(?m)^#include "([A-Za-z0-9_]+)\.h""#).expect("regex");
    include
        .captures_iter(source)
        .map(|c| c[1].to_string())
        .collect()
}

/// The functions the sibling headers `headers` include declare: the symbols of
/// the documents an importing document calls. C has no way to bring a function
/// in under another name, so a local an author gives the name of one hides it
/// for the rest of the function; that is the limit of what a name can be in C
/// (see the note where the candidates are chosen), and these are counted, not
/// asked.
fn imported_symbols(proj: &Path, headers: &[Header]) -> BTreeSet<String> {
    let function =
        Regex::new(r"(?m)^static inline [^;{(]*?\b([A-Za-z_][A-Za-z0-9_]*)\(").expect("regex");
    let mut symbols = BTreeSet::new();
    for header in headers {
        for sibling in sibling_includes(&header.source) {
            if let Ok(source) = std::fs::read_to_string(proj.join(format!("{sibling}.h"))) {
                symbols.extend(function.captures_iter(&source).map(|c| c[1].to_string()));
            }
        }
    }
    symbols
}

/// Write `headers`, and before each the sibling headers it includes, every one
/// generated from its own document under its own name (the include is the
/// document's, so a renamed sibling would not be found). It is the importing
/// document's names that are asked, never the sibling's.
fn write_with_siblings(
    proj: &Path,
    written: &mut BTreeSet<String>,
    headers: &[Header],
) -> Result<(), String> {
    for header in headers {
        for sibling in sibling_includes(&header.source) {
            let file = format!("{sibling}.h");
            if written.contains(&file) {
                continue;
            }
            let path = resource_dir().join(format!("{sibling}.scxml"));
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("the sibling `{sibling}` is not a document here: {e}"))?;
            let generated = generate(&sibling, &text)?;
            write_with_siblings(proj, written, &generated)?;
        }
        if written.insert(header.file.clone()) {
            std::fs::write(proj.join(&header.file), &header.source).expect("write a header");
        }
    }
    Ok(())
}

/// Compile `probe` (a translation unit that includes the headers under test)
/// and answer with the compiler's first complaint, if it has one.
fn compile(cc: &Path, proj: &Path, probe: &str) -> Option<String> {
    let mut command = Command::new(cc);
    command
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"])
        .arg("-I")
        .arg(proj);
    for include in c_runtime_includes() {
        command.arg("-I").arg(include);
    }
    let out = command
        .arg(format!("{probe}.c"))
        .current_dir(proj)
        .output()
        .expect("the C compiler runs");
    if out.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let first = stderr
        .lines()
        .find(|l| l.contains("error:"))
        .or_else(|| stderr.lines().next())
        .unwrap_or("the compiler failed without saying why");
    Some(first.trim().to_string())
}

/// Write the probe for each header set and compile them all.
fn compile_all(cc: &Path, proj: &Path, uniques: &[String]) -> BTreeMap<String, String> {
    run_parallel(uniques, |unique| {
        compile(cc, proj, &format!("probe_{unique}"))
    })
}

struct Case {
    kind: String,
    stem: String,
    declared: String,
    candidate: String,
    unique: String,
}

#[test]
fn an_authors_name_never_decides_whether_the_generated_c_of_a_kind_builds() {
    let Some(cc) = sce_build::toolchain::require_any_or_skip(
        &["clang", "gcc", "cc"],
        "the C kind name oracle",
    ) else {
        return;
    };
    let docs = documents(&resource_dir());
    assert!(
        docs.len() >= 40,
        "implausibly few documents of the eight kinds ({}); the scan broke",
        docs.len()
    );
    let from_outputs = names_by_kind(&expected_dir(), &["c.h"], Lang::CFamily);
    assert!(
        from_outputs.len() >= 6,
        "the committed C outputs name only {} of the kinds; the derivation broke",
        from_outputs.len()
    );

    let proj = std::env::temp_dir().join(format!("sce_c_kind_names_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&proj);
    std::fs::create_dir_all(&proj).expect("mkdir");
    let mut written: BTreeSet<String> = BTreeSet::new();

    // A probe is the translation unit that includes the headers under test.
    let write_probe = |unique: &str, headers: &[Header]| {
        let includes: String = headers
            .iter()
            .map(|h| format!("#include \"{}\"\n", h.file))
            .collect();
        std::fs::write(proj.join(format!("probe_{unique}.c")), includes).expect("write a probe");
    };

    // The unrenamed documents are the control: each must build, or a failure of
    // its renamings is not about the name.
    let mut baseline: Vec<(String, String)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut symbols: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for doc in &docs {
        let unique = format!("{}__base", doc.stem);
        match generate(&unique, &doc.text).and_then(|headers| {
            write_with_siblings(&proj, &mut written, &headers).map(|()| headers)
        }) {
            Ok(headers) => {
                write_probe(&unique, &headers);
                symbols.insert(doc.stem.clone(), imported_symbols(&proj, &headers));
                baseline.push((doc.stem.clone(), unique));
            }
            Err(_) => skipped.push(doc.stem.clone()),
        }
    }
    let control_uniques: Vec<String> = baseline.iter().map(|(_, u)| u.clone()).collect();
    let control_broken = compile_all(&cc, &proj, &control_uniques);
    let mut unbuildable: Vec<(String, String)> = Vec::new();
    baseline.retain(|(stem, unique)| match control_broken.get(unique) {
        Some(why) => {
            unbuildable.push((stem.clone(), why.clone()));
            false
        }
        None => true,
    });
    eprintln!("c kind name oracle: controls that do not build as C: {unbuildable:?}");
    skipped.extend(unbuildable.iter().map(|(stem, _)| stem.clone()));

    let mut cases: Vec<Case> = Vec::new();
    let mut attempts = 0usize;
    let mut refused = 0usize;
    let mut folded_together = 0usize;
    let mut left_out_symbols = 0usize;
    let mut other_refusals: BTreeMap<String, usize> = BTreeMap::new();
    for doc in &docs {
        if skipped.contains(&doc.stem) {
            continue;
        }
        let names = candidates(doc, &from_outputs, UNIVERSE);
        let folded: BTreeSet<String> = doc.declared.iter().map(|n| snake(n)).collect();
        let imported = &symbols[&doc.stem];
        for declared in &doc.declared {
            for candidate in &names {
                if candidate == declared || doc.declared.contains(candidate) {
                    continue;
                }
                // The name of a function the document imports: C cannot bring a
                // function in under another name, so this is what a name can be
                // in C and not a defect of the generator's spelling. Counted,
                // and said in the totals, so that it is a limit and not a
                // silence.
                if imported.contains(candidate) {
                    left_out_symbols += 1;
                    continue;
                }
                // Two author names that fold to one spelling are a different
                // defect — two of the AUTHOR's names meeting — and are counted,
                // not asked of this oracle.
                if folded.contains(&snake(candidate)) {
                    folded_together += 1;
                    continue;
                }
                attempts += 1;
                let renamed = rename(&doc.text, declared, candidate);
                // `c<n>`, not a bare number: a file name is a name, and none of
                // ours is allowed to be one the toolchain reads for itself.
                let unique = format!("{}__c{}", doc.stem, cases.len());
                match generate(&unique, &renamed) {
                    Err(why) => {
                        refused += 1;
                        if !is_name_refusal(&why) {
                            let key: String = why.chars().take(70).collect();
                            *other_refusals.entry(key).or_default() += 1;
                        }
                    }
                    Ok(headers) => {
                        write_with_siblings(&proj, &mut written, &headers)
                            .expect("a sibling that built for the control builds for a case");
                        write_probe(&unique, &headers);
                        cases.push(Case {
                            kind: doc.kind.clone(),
                            stem: doc.stem.clone(),
                            declared: declared.clone(),
                            candidate: candidate.clone(),
                            unique,
                        });
                    }
                }
            }
        }
    }

    let uniques: Vec<String> = cases.iter().map(|c| c.unique.clone()).collect();
    let broken = compile_all(&cc, &proj, &uniques);
    let _ = std::fs::remove_dir_all(&proj);

    let failures: Vec<Failure> = cases
        .iter()
        .filter_map(|case| {
            broken.get(&case.unique).map(|why| Failure {
                kind: case.kind.clone(),
                stem: case.stem.clone(),
                declared: case.declared.clone(),
                candidate: case.candidate.clone(),
                why: why.clone(),
            })
        })
        .collect();
    eprintln!(
        "c kind name oracle: {} documents ({} not renamed), {attempts} renamings tried, {refused} \
         refused, {folded_together} left out because two of the author's own names fold to one, \
         {left_out_symbols} left out because the name is a function the document imports, \
         {} built, {} did not build",
        docs.len(),
        skipped.len(),
        cases.len(),
        failures.len()
    );
    eprintln!("c kind name oracle: not renamed (refused unrenamed, or a control that does not build): {skipped:?}");
    assert!(
        cases.len() * 10 >= attempts * 7,
        "only {} of {attempts} renamings generated ({refused} refused); the oracle is mostly \
         asking nothing. Refusals not about a name: {other_refusals:?}",
        cases.len()
    );
    assert!(
        cases.len() >= 2000,
        "only {} renamings were built; not renamed: {skipped:?}",
        cases.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {} accepted renamings make the generated C not build ({attempts} tried, {refused} \
         refused, {folded_together} left out because two of the author's own names fold to \
         one):\n{}",
        failures.len(),
        cases.len(),
        group(&failures).join("\n")
    );
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
    let from_outputs = names_by_kind(&expected_dir(), &["c.h"], Lang::CFamily);
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
