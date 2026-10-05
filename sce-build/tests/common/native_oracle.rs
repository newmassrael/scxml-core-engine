// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The name oracle for the backends whose generated code is a HEADER that a
//! translation unit includes, and a compiler checks: C and C++.
//!
//! What a backend differs in is a [`Native`]: the language it generates, the
//! committed outputs its candidate names are read from, its compiler and the
//! flags that hold its output to the contract, and whether it can call an
//! imported function through a qualifier (C++ can, C cannot). [`run`] is the
//! rest: every document of the eight kinds is generated with each name it
//! declares renamed to each candidate, written beside the sibling headers it
//! includes, and compiled as a translation unit. A name the generator refuses is
//! an answer. What may not happen is a name that is accepted and then makes the
//! generated code not compile.
//!
//! The generated code is compiled, not run: a name that is accepted, compiles
//! and does something else is not seen here.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

use super::name_oracle::{
    candidates, documents, group, is_name_refusal, names_by_kind, rename, run_parallel, snake,
    with_name, Failure,
};
use super::source_lexing::Lang;

/// A generated header: the file name it is written under and its source.
pub struct Header {
    pub file: String,
    pub source: String,
}

/// Everything that differs between two header-and-translation-unit backends.
pub struct Native {
    /// What the oracle calls itself where it speaks (`c`, `c++`).
    pub label: &'static str,
    pub language: Language,
    pub resource_dir: PathBuf,
    pub expected_dir: PathBuf,
    /// The committed outputs the candidate names are read from, and the suffixes
    /// of another backend that end the same way.
    pub output_extensions: &'static [&'static str],
    pub excluded_suffixes: &'static [&'static str],
    pub lang: Lang,
    /// The language's own names an author meets, which it and the templates fix.
    pub universe: &'static [&'static str],
    /// The extension of the translation unit that includes the headers.
    pub probe_extension: &'static str,
    /// The compilers that may stand for it, in the order they are tried.
    pub compilers: &'static [&'static str],
    /// The flags of the compile of one probe, and the include directories the
    /// generated headers need beside the one they are written to.
    pub compile_flags: &'static [&'static str],
    pub include_dirs: Vec<PathBuf>,
    /// A pattern whose first group is the name of a function a header declares,
    /// where the language cannot call an imported function through a qualifier
    /// and so cannot keep an author's name off it (C): such a name is counted and
    /// not asked. `None` where a call is qualified and the question has no such
    /// case.
    pub imported_function: Option<&'static str>,
}

/// Generate `text` under the unique name `unique`: the headers, or the reason it
/// was refused.
fn generate(native: &Native, unique: &str, text: &str) -> Result<Vec<Header>, String> {
    compile_forge_with_imports(
        &with_name(text, unique),
        DocumentLabel::symmetric(unique),
        native.language,
        &native.resource_dir,
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

/// The functions the sibling headers `headers` include declare, when the
/// language has the limit [`Native::imported_function`] names.
fn imported_symbols(native: &Native, proj: &Path, headers: &[Header]) -> BTreeSet<String> {
    let Some(pattern) = native.imported_function else {
        return BTreeSet::new();
    };
    let function = Regex::new(pattern).expect("regex");
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
    native: &Native,
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
            let path = native.resource_dir.join(format!("{sibling}.scxml"));
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("the sibling `{sibling}` is not a document here: {e}"))?;
            let generated = generate(native, &sibling, &text)?;
            write_with_siblings(native, proj, written, &generated)?;
        }
        if written.insert(header.file.clone()) {
            std::fs::write(proj.join(&header.file), &header.source).expect("write a header");
        }
    }
    Ok(())
}

/// Compile one probe and answer with the compiler's first complaint, if it has
/// one.
fn compile(native: &Native, cc: &Path, proj: &Path, probe: &str) -> Option<String> {
    let mut command = Command::new(cc);
    command.args(native.compile_flags).arg("-I").arg(proj);
    for include in &native.include_dirs {
        command.arg("-I").arg(include);
    }
    let out = command
        .arg(format!("{probe}.{}", native.probe_extension))
        .current_dir(proj)
        .output()
        .expect("the compiler runs");
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

/// Compile every probe.
fn compile_all(
    native: &Native,
    cc: &Path,
    proj: &Path,
    uniques: &[String],
) -> BTreeMap<String, String> {
    run_parallel(uniques, |unique| {
        compile(native, cc, proj, &format!("probe_{unique}"))
    })
}

struct Case {
    kind: String,
    stem: String,
    declared: String,
    candidate: String,
    unique: String,
}

/// Ask the question of one backend; panics with the failures grouped by cause.
pub fn run(native: &Native) {
    let label = native.label;
    let Some(cc) = sce_build::toolchain::require_any_or_skip(
        native.compilers,
        &format!("the {label} kind name oracle"),
    ) else {
        return;
    };
    let docs = documents(&native.resource_dir);
    assert!(
        docs.len() >= 40,
        "implausibly few documents of the eight kinds ({}); the scan broke",
        docs.len()
    );
    let from_outputs = names_by_kind(
        &native.expected_dir,
        native.output_extensions,
        native.excluded_suffixes,
        native.lang,
    );
    assert!(
        from_outputs.len() >= 6,
        "the committed {label} outputs name only {} of the kinds; the derivation broke",
        from_outputs.len()
    );

    let proj = std::env::temp_dir().join(format!(
        "sce_{}_kind_names_{}",
        label.replace('+', "p"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&proj);
    std::fs::create_dir_all(&proj).expect("mkdir");
    let mut written: BTreeSet<String> = BTreeSet::new();

    // A probe is the translation unit that includes the headers under test.
    let write_probe = |unique: &str, headers: &[Header]| {
        let includes: String = headers
            .iter()
            .map(|h| format!("#include \"{}\"\n", h.file))
            .collect();
        std::fs::write(
            proj.join(format!("probe_{unique}.{}", native.probe_extension)),
            includes,
        )
        .expect("write a probe");
    };

    // The unrenamed documents are the control: each must build, or a failure of
    // its renamings is not about the name.
    let mut baseline: Vec<(String, String)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut symbols: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for doc in &docs {
        let unique = format!("{}__base", doc.stem);
        match generate(native, &unique, &doc.text).and_then(|headers| {
            write_with_siblings(native, &proj, &mut written, &headers).map(|()| headers)
        }) {
            Ok(headers) => {
                write_probe(&unique, &headers);
                symbols.insert(doc.stem.clone(), imported_symbols(native, &proj, &headers));
                baseline.push((doc.stem.clone(), unique));
            }
            Err(_) => skipped.push(doc.stem.clone()),
        }
    }
    let control_uniques: Vec<String> = baseline.iter().map(|(_, u)| u.clone()).collect();
    let control_broken = compile_all(native, &cc, &proj, &control_uniques);
    let mut unbuildable: Vec<(String, String)> = Vec::new();
    baseline.retain(|(stem, unique)| match control_broken.get(unique) {
        Some(why) => {
            unbuildable.push((stem.clone(), why.clone()));
            false
        }
        None => true,
    });
    eprintln!("{label} kind name oracle: controls that do not build: {unbuildable:?}");
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
        let names = candidates(doc, &from_outputs, native.universe);
        let folded: BTreeSet<String> = doc.declared.iter().map(|n| snake(n)).collect();
        let imported = &symbols[&doc.stem];
        for declared in &doc.declared {
            for candidate in &names {
                if candidate == declared || doc.declared.contains(candidate) {
                    continue;
                }
                // The name of a function the document imports, where the
                // language cannot call it through a qualifier: this is what a
                // name can be there and not a defect of the generator's
                // spelling. Counted, and said in the totals, so that it is a
                // limit and not a silence.
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
                match generate(native, &unique, &renamed) {
                    Err(why) => {
                        refused += 1;
                        if !is_name_refusal(&why) {
                            let key: String = why.chars().take(70).collect();
                            *other_refusals.entry(key).or_default() += 1;
                        }
                    }
                    Ok(headers) => {
                        write_with_siblings(native, &proj, &mut written, &headers)
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
    let broken = compile_all(native, &cc, &proj, &uniques);
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
        "{label} kind name oracle: {} documents ({} not renamed), {attempts} renamings tried, \
         {refused} refused, {folded_together} left out because two of the author's own names fold \
         to one, {left_out_symbols} left out because the name is a function the document \
         imports, {} built, {} did not build",
        docs.len(),
        skipped.len(),
        cases.len(),
        failures.len()
    );
    eprintln!(
        "{label} kind name oracle: not renamed (refused unrenamed, or a control that does not \
         build): {skipped:?}"
    );
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
        "{} of {} accepted renamings make the generated {label} not build ({attempts} tried, \
         {refused} refused, {folded_together} left out because two of the author's own names \
         fold to one):\n{}",
        failures.len(),
        cases.len(),
        group(&failures).join("\n")
    );
}
