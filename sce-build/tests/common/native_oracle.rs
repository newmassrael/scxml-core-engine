// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The name oracle for the backends whose generated code is a UNIT that a
//! probe pulls in and a compiler checks: C and C++ (a header a translation unit
//! includes) and Rust (a module a crate root declares).
//!
//! What a backend differs in is a [`Native`]: the language it generates, the
//! committed outputs its candidate names are read from, its compiler and the
//! flags that hold its output to the contract, how a unit names a sibling and
//! how a probe pulls one in, and whether it can call an imported function
//! through a qualifier (C++ and Rust can, C cannot). [`run`] is the rest: every
//! document of the eight kinds is generated with each name it declares renamed
//! to each candidate, written beside the sibling units it names, and compiled
//! as a probe. A name the generator refuses is an answer. What may not happen
//! is a name that is accepted and then makes the generated code not compile.
//!
//! The generated code is compiled, not run: a name that is accepted, compiles
//! and does something else is not seen here.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use regex::Regex;
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

use super::name_oracle::{
    candidates, documents, group, identifiers, is_name_refusal, names_by_kind, rename,
    run_parallel, run_parallel_on, snake, with_name, Failure,
};
use super::source_lexing::Lang;

/// A generated unit: the file name it is written under and its source.
pub struct Header {
    pub file: String,
    pub source: String,
}

/// How a C or C++ unit names a sibling: `#include "condition_threshold.h"`, as
/// opposed to the runtime's (`sce/forge/…`).
pub const INCLUDE_SIBLING: &str = r#"(?m)^#include "([A-Za-z0-9_]+)\.h""#;

/// The probe line of a C or C++ unit: the include of its header.
pub fn include_probe_line(file: &str) -> String {
    format!("#include \"{file}\"\n")
}

/// A compiler that is slow to start (a JVM) is run once on many units, and what
/// it says is told back to the unit it is about by the file it names.
pub struct Batch {
    /// How many cases one compiler run holds.
    pub per_run: usize,
    /// How many compiler runs are at once: a JVM holds gigabytes, so the
    /// machine's cores are not the limit.
    pub threads: usize,
    /// The flag that names the directory the compiler writes its output to
    /// (`-d`), one for each run.
    pub output_flag: &'static str,
    /// A pattern for one diagnostic, whose first group is the file it is about
    /// and whose second is what it says (`file.kt:1:2: error: why`). A warning
    /// is one too, where the contract is that there are none.
    pub diagnostic: &'static str,
    /// The environment of the compiler (`JAVA_OPTS`, the heap it may take).
    pub env: &'static [(&'static str, &'static str)],
}

/// Everything that differs between two unit-and-probe backends.
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
    /// The flag that names an include directory (`-I`), or `None` where the
    /// compiler has no such flag and finds a sibling by the probe (Rust).
    pub include_flag: Option<&'static str>,
    /// Flags that are known only at run time, after the compile of what the
    /// units need (the runtime crates a Rust unit names), placed after
    /// `compile_flags`.
    pub extra_flags: Vec<String>,
    /// The suffix of a generated unit (`.h`, `.rs`): what of the generator's
    /// output the oracle writes and compiles.
    pub unit_suffix: &'static str,
    /// A pattern whose first group is the name of a sibling unit a generated unit
    /// names ([`INCLUDE_SIBLING`]).
    pub sibling_pattern: &'static str,
    /// The file a document's unit is written to, where it is not the document's
    /// name and the suffix (Kotlin writes `ConditionThreshold.kt`).
    pub file_name: Option<fn(&str) -> String>,
    /// The line a probe holds to pull one unit in ([`include_probe_line`]).
    pub probe_line: fn(&str) -> String,
    /// A compiler run once on many units, with no probe, or `None` for one run
    /// of the compiler on one probe.
    pub batch: Option<Batch>,
    /// A pattern whose first group is the name of a function a header declares,
    /// where the language cannot call an imported function through a qualifier
    /// and so cannot keep an author's name off it (C): such a name is counted and
    /// not asked. `None` where a call is qualified and the question has no such
    /// case.
    pub imported_function: Option<&'static str>,
}

impl Native {
    /// The file the unit of the document `stem` is written to.
    fn file_of(&self, stem: &str) -> String {
        match self.file_name {
            Some(file_name) => file_name(stem),
            None => format!("{stem}{}", self.unit_suffix),
        }
    }
}

/// Generate `text` under the unique name `unique`: the units, or the reason it
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
            .filter(|(file, _)| file.ends_with(native.unit_suffix))
            .map(|(file, source)| Header { file, source })
            .collect()
    })
}

/// The sibling units a generated unit names (`#include "condition_threshold.h"`,
/// `use super::condition_threshold`), as opposed to the runtime's.
fn sibling_includes(native: &Native, source: &str) -> Vec<String> {
    let sibling = Regex::new(native.sibling_pattern).expect("regex");
    sibling
        .captures_iter(source)
        .map(|c| c[1].to_string())
        .collect()
}

/// The functions the sibling units `headers` name declare, when the language has
/// the limit [`Native::imported_function`] names.
fn imported_symbols(native: &Native, proj: &Path, headers: &[Header]) -> BTreeSet<String> {
    let Some(pattern) = native.imported_function else {
        return BTreeSet::new();
    };
    let function = Regex::new(pattern).expect("regex");
    let mut symbols = BTreeSet::new();
    for header in headers {
        for sibling in sibling_includes(native, &header.source) {
            if let Ok(source) = std::fs::read_to_string(proj.join(native.file_of(&sibling))) {
                symbols.extend(function.captures_iter(&source).map(|c| c[1].to_string()));
            }
        }
    }
    symbols
}

/// Write `headers`, and before each the sibling units it names, every one
/// generated from its own document under its own name (the name is the
/// document's, so a renamed sibling would not be found). It is the importing
/// document's names that are asked, never the sibling's.
fn write_with_siblings(
    native: &Native,
    proj: &Path,
    written: &mut BTreeSet<String>,
    headers: &[Header],
) -> Result<(), String> {
    for header in headers {
        for sibling in sibling_includes(native, &header.source) {
            let file = native.file_of(&sibling);
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
            std::fs::write(proj.join(&header.file), &header.source).expect("write a unit");
        }
    }
    Ok(())
}

/// Every file `headers` need, themselves and the siblings they name, however
/// deep, read from what is written: a probe pulls in what a language does not
/// pull in by itself (a Rust crate root declares every module of the crate).
fn needed_files(native: &Native, proj: &Path, headers: &[Header]) -> Vec<String> {
    let mut needed: Vec<String> = headers.iter().map(|h| h.file.clone()).collect();
    let mut next = 0;
    while next < needed.len() {
        let source = std::fs::read_to_string(proj.join(&needed[next])).unwrap_or_default();
        for sibling in sibling_includes(native, &source) {
            let file = native.file_of(&sibling);
            if !needed.contains(&file) {
                needed.push(file);
            }
        }
        next += 1;
    }
    needed
}

/// Compile one probe and answer with the compiler's first complaint, if it has
/// one.
fn compile(native: &Native, cc: &Path, proj: &Path, probe: &str) -> Option<String> {
    let mut command = Command::new(cc);
    command.args(native.compile_flags);
    if let Some(flag) = native.include_flag {
        command.arg(flag).arg(proj);
        for include in &native.include_dirs {
            command.arg(flag).arg(include);
        }
    }
    let out = command
        .args(&native.extra_flags)
        .arg(format!("{probe}.{}", native.probe_extension))
        .current_dir(proj)
        .output()
        .expect("the compiler runs");
    if out.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    // A C compiler says `file:1:2: error: why`; rustc says `error: why` or, for
    // an error with a code, `error[E0425]: why` and ends with a count of them
    // (`error: aborting due to 2 previous errors`), which is not a cause.
    let first = stderr
        .lines()
        .find(|l| {
            (l.contains("error:") || l.starts_with("error[")) && !l.contains("aborting due to")
        })
        .or_else(|| stderr.lines().next())
        .unwrap_or("the compiler failed without saying why");
    Some(first.trim().to_string())
}

/// What one case needs compiled: the files it wrote itself, which an error is
/// about, and every file the compile needs, those and the siblings they name.
struct Plan {
    unique: String,
    own: Vec<String>,
    needed: Vec<String>,
}

/// Compile every plan; answers with the reason for each that does not build.
fn compile_all(
    native: &Native,
    cc: &Path,
    proj: &Path,
    plans: &[Plan],
) -> BTreeMap<String, String> {
    match &native.batch {
        None => {
            let uniques: Vec<String> = plans.iter().map(|p| p.unique.clone()).collect();
            run_parallel(&uniques, |unique| {
                compile(native, cc, proj, &format!("probe_{unique}"))
            })
        }
        Some(batch) => compile_in_batches(native, batch, cc, proj, plans),
    }
}

/// Compile the plans a run at a time, one compiler process on all the units of a
/// run, and tell each error back to the case whose file it names. A diagnostic
/// about a file no case owns (a sibling every case shares) is not a case's; and a
/// run that failed without a diagnostic about any case is not read as a pass: it
/// stops the oracle, because what it says is that the compiler said nothing
/// about the units it was asked to judge.
fn compile_in_batches(
    native: &Native,
    batch: &Batch,
    cc: &Path,
    proj: &Path,
    plans: &[Plan],
) -> BTreeMap<String, String> {
    let owner: BTreeMap<&str, &str> = plans
        .iter()
        .flat_map(|p| {
            p.own
                .iter()
                .map(move |file| (file.as_str(), p.unique.as_str()))
        })
        .collect();
    let runs: Vec<&[Plan]> = plans.chunks(batch.per_run).collect();
    let items: Vec<String> = (0..runs.len()).map(|i| i.to_string()).collect();
    let diagnostic = Regex::new(batch.diagnostic).expect("regex");
    let answers: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
    let unattributed = run_parallel_on(&items, batch.threads, |item| {
        let index: usize = item.parse().expect("a run index");
        let files: BTreeSet<&str> = runs[index]
            .iter()
            .flat_map(|p| p.needed.iter().map(String::as_str))
            .collect();
        let mut command = Command::new(cc);
        command
            .args(native.compile_flags)
            .args(&native.extra_flags)
            .arg(batch.output_flag)
            .arg(proj.join(format!("out_{index}")));
        for (key, value) in batch.env {
            command.env(key, value);
        }
        let out = command
            .args(&files)
            .current_dir(proj)
            .output()
            .expect("the compiler runs");
        if out.status.success() {
            return None;
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        let mut about_a_case = false;
        for caps in diagnostic.captures_iter(&stderr) {
            let file = caps[1].rsplit('/').next().unwrap_or(&caps[1]).to_string();
            if let Some(unique) = owner.get(file.as_str()) {
                about_a_case = true;
                answers
                    .lock()
                    .expect("no run panics while holding the lock")
                    .entry((*unique).to_string())
                    .or_insert_with(|| caps[2].trim().to_string());
            }
        }
        if about_a_case {
            None
        } else {
            Some(stderr.lines().take(12).collect::<Vec<_>>().join("\n"))
        }
    });
    assert!(
        unattributed.is_empty(),
        "the compiler failed on a run of units and said nothing about any of them (a sibling \
         that does not build, or the compiler itself):\n{unattributed:#?}"
    );
    answers.into_inner().expect("no run panics")
}

/// Every identifier the generator writes for any document of the corpus, read
/// from the code it generates now and not from the committed outputs, which are
/// a subset of the corpus. What a pin test asks about the names a template
/// reaches for must be asked of all of them: a library function used by one
/// document with no committed output is a name nothing else would show.
pub fn generated_names(native: &Native) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for doc in documents(&native.resource_dir) {
        let unique = format!("{}_base", doc.stem);
        if let Ok(headers) = generate(native, &unique, &doc.text) {
            for header in &headers {
                names.extend(identifiers(&header.source, native.lang));
            }
        }
    }
    names
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

    // What one case is compiled from. A probe is the unit that pulls in what is
    // under test (the translation unit that includes the headers, the crate root
    // that declares the modules); a compiler that is run on the units themselves
    // has none.
    let plan_of = |unique: &str, headers: &[Header]| -> Plan {
        let needed = needed_files(native, &proj, headers);
        if native.batch.is_none() {
            let lines: String = needed
                .iter()
                .map(|file| (native.probe_line)(file))
                .collect();
            std::fs::write(
                proj.join(format!("probe_{unique}.{}", native.probe_extension)),
                lines,
            )
            .expect("write a probe");
        }
        Plan {
            unique: unique.to_string(),
            own: headers.iter().map(|h| h.file.clone()).collect(),
            needed,
        }
    };

    // The unrenamed documents are the control: each must build, or a failure of
    // its renamings is not about the name.
    let mut baseline_stems: Vec<String> = Vec::new();
    let mut baseline_plans: Vec<Plan> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut symbols: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // What the generator writes for a document itself, as read from its own
    // unrenamed output: a candidate for its renamings besides what the committed
    // outputs of its kind use, so that a name the generator derives from the
    // document's own (a member, a local) is asked about whether or not a
    // committed output still spells it.
    let mut own_names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for doc in &docs {
        let unique = format!("{}_base", doc.stem);
        match generate(native, &unique, &doc.text).and_then(|headers| {
            write_with_siblings(native, &proj, &mut written, &headers).map(|()| headers)
        }) {
            Ok(headers) => {
                baseline_plans.push(plan_of(&unique, &headers));
                symbols.insert(doc.stem.clone(), imported_symbols(native, &proj, &headers));
                // The document's own unique name is spelled into some of what it
                // writes (a package, a class): a name of the oracle's, not the
                // generator's.
                let mine = doc.stem.replace('_', "").to_lowercase();
                let written_names: BTreeSet<String> = headers
                    .iter()
                    .flat_map(|h| identifiers(&h.source, native.lang))
                    .filter(|n| !n.to_lowercase().replace('_', "").contains(&mine))
                    .collect();
                own_names.insert(doc.stem.clone(), written_names);
                baseline_stems.push(doc.stem.clone());
            }
            Err(_) => skipped.push(doc.stem.clone()),
        }
    }
    let control_broken = compile_all(native, &cc, &proj, &baseline_plans);
    let mut unbuildable: Vec<(String, String)> = Vec::new();
    for (stem, plan) in baseline_stems.iter().zip(&baseline_plans) {
        if let Some(why) = control_broken.get(&plan.unique) {
            unbuildable.push((stem.clone(), why.clone()));
        }
    }
    eprintln!("{label} kind name oracle: controls that do not build: {unbuildable:?}");
    skipped.extend(unbuildable.iter().map(|(stem, _)| stem.clone()));

    let mut cases: Vec<Case> = Vec::new();
    let mut case_plans: Vec<Plan> = Vec::new();
    let mut attempts = 0usize;
    let mut refused = 0usize;
    let mut folded_together = 0usize;
    let mut left_out_symbols = 0usize;
    let mut other_refusals: BTreeMap<String, usize> = BTreeMap::new();
    for doc in &docs {
        if skipped.contains(&doc.stem) {
            continue;
        }
        let mut names = candidates(doc, &from_outputs, native.universe);
        names.extend(own_names[&doc.stem].iter().cloned());
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
                // ours is allowed to be one the toolchain reads for itself. One
                // underscore, not two: a Rust module of that name is a snake-case
                // warning of the probe's, which `-D warnings` would make the
                // generated code's.
                let unique = format!("{}_c{}", doc.stem, cases.len());
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
                        case_plans.push(plan_of(&unique, &headers));
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

    let broken = compile_all(native, &cc, &proj, &case_plans);
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
