// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A C11 machine's core includes only what a freestanding implementation
//! provides (docs/adr/0110, D1): `<stdint.h>`, `<stddef.h>`, `<stdbool.h>`,
//! `<limits.h>`, `<float.h>`, `<stdarg.h>`, `<stdalign.h>`, `<stdnoreturn.h>`
//! and `<iso646.h>`, and the runtime's own headers that themselves include no
//! more. Everything hosted is a unit of its own, which a deployment links or
//! leaves out (D2), and is named here and not walked: a machine's saved unit
//! (`<machine>_saved.h`), the request reader of a typed host-run invocation,
//! which D2 keeps in the host's unit (`sce/request_field.h`), and the runtime of
//! the test profile a test component is driven by (`sce/test/`).
//!
//! What holds that here is the text a compiler reads: every `sce-static`
//! statechart among the static datamodel fixtures, and the documents the gate
//! of D3 weighs, generated as a build generates them, each machine's source
//! (`<machine>_sm.c`) walked include by include through the files the run
//! wrote and the runtime's two include directories. A header reached that is
//! neither freestanding nor the runtime's is named with the chain that reached
//! it, and so is an include that resolves nowhere, since a file the walk cannot
//! read is one it cannot judge. An include C++ alone reads (`#ifdef
//! __cplusplus`) is no part of a C core and is not walked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::RuntimeRequirement;

/// The headers C11 requires of a freestanding implementation (C11 clause 4,
/// paragraph 6), the only standard headers a core may include.
const FREESTANDING: [&str; 9] = [
    "stdint.h",
    "stddef.h",
    "stdbool.h",
    "limits.h",
    "float.h",
    "stdarg.h",
    "stdalign.h",
    "stdnoreturn.h",
    "iso646.h",
];

/// The documents the gate of docs/adr/0110, D3 links, weighs and runs, each of
/// which must generate and be walked whatever the rest of the fixtures do.
const GATE_SET: [&str; 5] = [
    "static_datamodel/static_counter.scxml",
    "no_std/plain_footprint_probe.scxml",
    "static_datamodel/static_map.scxml",
    "static_datamodel/static_map_take.scxml",
    "static_datamodel/static_state_algorithm.scxml",
];

/// The fewest machines the walk has to have judged, so a run that generated
/// almost nothing is no pass: the static datamodel fixtures hold over a
/// hundred `sce-static` statecharts, and those the generator refuses (the
/// refusal fixtures, and a host-run invocation generated without the host's
/// declaration) are a minority.
const FEWEST_MACHINES: usize = 50;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The runtime's include directories, in the order a build searches them.
fn include_dirs() -> [PathBuf; 2] {
    [
        repo_root().join("backends/c/runtime/include"),
        repo_root().join("backends/c/forge-runtime/include"),
    ]
}

/// Whether the header `name`, as an include names it, is a hosted unit
/// docs/adr/0110, D2 places outside every core, which the walk does not
/// descend into.
fn hosted_unit(name: &str) -> bool {
    name.ends_with("_saved.h") || name == "sce/request_field.h" || name.starts_with("sce/test/")
}

/// What one generation wrote: its C11 files by file name, and what its
/// manifest states its machines require (`runtime_requires`).
struct Generation {
    files: BTreeMap<String, String>,
    requires: Vec<String>,
}

/// One generation of `document` into `out`, or `None` when the generator
/// refused the document.
fn generate(document: &Path, out: &Path) -> Option<Generation> {
    let input_root = document.parent().expect("a fixture has a directory");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("generate")
        .arg(document)
        .args(["-l", "c11", "-o"])
        .arg(out)
        .arg("--input-root")
        .arg(input_root)
        .args(["--with-imports", "--no-format"])
        .output()
        .expect("spawn sce-codegen");
    if !run.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&run.stdout);
    let manifest: serde_json::Value = serde_json::from_str(
        stdout
            .lines()
            .last()
            .unwrap_or_else(|| panic!("{} wrote no manifest", document.display())),
    )
    .unwrap_or_else(|e| panic!("{}: the manifest is not JSON: {e}", document.display()));
    let requires = manifest["runtime_requires"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "{}: a C11 manifest states runtime_requires",
                document.display()
            )
        })
        .iter()
        .map(|v| v.as_str().expect("a requirement is a string").to_string())
        .collect();
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(out).expect("read the output directory") {
        let path = entry.expect("an output file").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a UTF-8 file name")
            .to_string();
        if name.ends_with(".c") || name.ends_with(".h") {
            files.insert(
                name,
                std::fs::read_to_string(&path).expect("read a generated file"),
            );
        }
    }
    Some(Generation { files, requires })
}

/// One `#include` of a text: what it names, and whether it is quoted.
struct Include {
    name: String,
    quoted: bool,
}

/// The includes of `text` a C compiler reads: those inside a branch that only
/// C++ takes (`#ifdef __cplusplus`, `#if defined(__cplusplus)` and the `#else`
/// of their negations) are left out, and every other branch is read, since
/// the walk cannot know which a build takes.
fn includes(text: &str) -> Vec<Include> {
    // For each open conditional, whether it tests `__cplusplus`, and whether
    // its current branch is one C reads.
    let mut frames: Vec<(bool, bool)> = Vec::new();
    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        let Some(directive) = line.strip_prefix('#') else {
            continue;
        };
        let directive = directive.trim_start();
        let word: String = directive
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let rest = directive[word.len()..].trim();
        match word.as_str() {
            "if" | "ifdef" | "ifndef" => {
                let negated = word == "ifndef" || rest.starts_with('!');
                let on_cplusplus = rest.contains("__cplusplus");
                // A branch C reads: any that does not test `__cplusplus`, and
                // the negation of one that does.
                frames.push((on_cplusplus, !on_cplusplus || negated));
            }
            "else" | "elif" => {
                if let Some((on_cplusplus, read)) = frames.last_mut() {
                    if *on_cplusplus {
                        *read = !*read;
                    }
                }
            }
            "endif" => {
                frames.pop();
            }
            "include" if frames.iter().all(|(_, read)| *read) => {
                let (quoted, close) = match rest.chars().next() {
                    Some('"') => (true, '"'),
                    Some('<') => (false, '>'),
                    _ => continue,
                };
                if let Some(end) = rest[1..].find(close) {
                    found.push(Include {
                        name: rest[1..1 + end].to_string(),
                        quoted,
                    });
                }
            }
            _ => {}
        }
    }
    found
}

/// What the walk from one machine's source found.
#[derive(Default)]
struct Reached {
    /// Every hosted header, or include that resolves nowhere, each with the
    /// chain that reached it.
    violations: Vec<String>,
    /// The header of another machine (`<machine>_sm.h`, a child this one
    /// starts) that this generation did not write: its own generation writes
    /// it, and the walk of that generation judges it.
    other_machines: Vec<String>,
}

/// What the source `core` reaches through `files` (the generation's own) and
/// the runtime's include directories.
fn hosted_reached(core: &str, files: &BTreeMap<String, String>) -> Reached {
    let dirs = include_dirs();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut reached = Reached::default();
    let mut pending: Vec<(String, Vec<String>)> =
        vec![(files[core].clone(), vec![core.to_string()])];
    while let Some((text, chain)) = pending.pop() {
        for include in includes(&text) {
            let name = include.name;
            if hosted_unit(&name) {
                continue;
            }
            let mut next = chain.clone();
            next.push(name.clone());
            // A generated file first, as the compiler finds a quoted include
            // beside the file that names it; then the runtime's directories.
            let generated = if include.quoted {
                Path::new(&name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| files.get(n).map(|text| (n.to_string(), text.clone())))
            } else {
                None
            };
            let resolved = generated.or_else(|| {
                dirs.iter().find_map(|dir| {
                    let path = dir.join(&name);
                    std::fs::read_to_string(&path)
                        .ok()
                        .map(|text| (path.display().to_string(), text))
                })
            });
            match resolved {
                Some((key, text)) => {
                    if seen.insert(key) {
                        pending.push((text, next));
                    }
                }
                None if !include.quoted && FREESTANDING.contains(&name.as_str()) => {}
                None if !include.quoted => reached
                    .violations
                    .push(format!("hosted <{name}>: {}", next.join(" > "))),
                None if name.ends_with("_sm.h") => reached.other_machines.push(name),
                None => reached.violations.push(format!(
                    "\"{name}\" resolves to no file the walk can read: {}",
                    next.join(" > ")
                )),
            }
        }
    }
    reached
}

/// docs/adr/0110, D1: no machine's core reaches a hosted header.
#[test]
fn a_c11_machines_core_reaches_only_freestanding_headers() {
    let mut documents: Vec<PathBuf> = Vec::new();
    let static_dir = fixtures().join("static_datamodel");
    for entry in std::fs::read_dir(&static_dir).expect("read the static datamodel fixtures") {
        let path = entry.expect("a fixture").path();
        if path.extension().is_some_and(|e| e == "scxml")
            && std::fs::read_to_string(&path)
                .expect("read a fixture")
                .contains("datamodel=\"sce-static\"")
        {
            documents.push(path);
        }
    }
    for document in GATE_SET {
        let path = fixtures().join(document);
        if !documents.contains(&path) {
            documents.push(path);
        }
    }
    documents.sort();

    let gate_set: BTreeSet<PathBuf> = GATE_SET.iter().map(|d| fixtures().join(d)).collect();
    let mut violations: Vec<String> = Vec::new();
    let mut walked = 0usize;
    let mut refused: Vec<String> = Vec::new();
    // Each machine walked, by its header, and the headers of other machines a
    // walk met that its generation did not write, with what met them.
    let mut walked_headers: BTreeSet<String> = BTreeSet::new();
    let mut met_elsewhere: Vec<(String, String)> = Vec::new();
    let mut stating_real = 0usize;
    for document in &documents {
        let out = tempfile::tempdir().expect("an output directory");
        let name = document
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a UTF-8 fixture name")
            .to_string();
        let Some(Generation { files, requires }) = generate(document, out.path()) else {
            assert!(
                !gate_set.contains(document),
                "{name}, which the gate of docs/adr/0110 D3 weighs, does not generate"
            );
            refused.push(name);
            continue;
        };
        // docs/adr/0110, D1: a machine whose core hands a real to the seams
        // states `real`, so its build links what defines them, exactly when it
        // includes them. The conformance registry's machines are ECMAScript
        // statecharts that hand none
        // (`a_c11_machine_includes_what_it_states_it_requires`); these are the
        // corpus that states it.
        let real = RuntimeRequirement::Real;
        let stated = requires.iter().any(|r| r == real.wire_name());
        let included = files
            .values()
            .any(|text| text.lines().any(|line| line.trim() == real.c11_include()));
        if stated != included {
            violations.push(format!(
                "{name}: states `real` {stated} and includes `{}` {included}",
                real.c11_include()
            ));
        }
        if stated {
            stating_real += 1;
        }
        let cores: Vec<&String> = files.keys().filter(|f| f.ends_with("_sm.c")).collect();
        assert!(
            !gate_set.contains(document) || !cores.is_empty(),
            "{name} wrote no machine: {:?}",
            files.keys().collect::<Vec<_>>()
        );
        for core in cores {
            walked += 1;
            walked_headers.insert(format!("{}.h", core.trim_end_matches(".c")));
            let reached = hosted_reached(core, &files);
            for violation in reached.violations {
                violations.push(format!("{name}: {violation}"));
            }
            for other in reached.other_machines {
                met_elsewhere.push((name.clone(), other));
            }
        }
    }
    // A machine's header another generation writes is judged by the walk of
    // that generation, and one no walk judged is a file nothing read.
    for (name, other) in met_elsewhere {
        if !walked_headers.contains(&other) {
            violations.push(format!(
                "{name}: \"{other}\", another machine's header, is the header of no machine \
                 the walk judged"
            ));
        }
    }
    // Every `sce-static` machine writes its events' data through the wire
    // writer, which hands a real to the seam, so the corpus states `real`.
    assert!(
        stating_real >= FEWEST_MACHINES,
        "only {stating_real} generations state `real`, so the comparison of what a machine \
         states with what it includes measured little"
    );
    assert!(
        walked >= FEWEST_MACHINES,
        "the walk judged {walked} machines of {} documents, fewer than {FEWEST_MACHINES}; the \
         generator refused {refused:?}",
        documents.len()
    );
    assert!(
        violations.is_empty(),
        "a machine's core reaches what a freestanding implementation does not provide \
         (docs/adr/0110, D1):\n{}",
        violations.join("\n")
    );
}

/// The walk itself: an include only C++ reads is not walked, a hosted header
/// a C branch names is found, and a hosted unit is not descended into.
#[test]
fn the_walk_reads_the_includes_a_c_compiler_reads() {
    let text = "#ifdef __cplusplus\n#include <cstring>\n#else\n#include <stdint.h>\n#endif\n\
                #if defined(__cplusplus)\nextern \"C\" {\n#endif\n#if SCE_MAX_INBOX > 0\n\
                #include <stdio.h>\n#endif\n#include \"m_saved.h\"\n";
    let names: Vec<String> = includes(text).into_iter().map(|i| i.name).collect();
    assert_eq!(names, ["stdint.h", "stdio.h", "m_saved.h"]);

    let files: BTreeMap<String, String> = [
        (
            "m_sm.c".to_string(),
            "#include \"m_sm.h\"\n#include \"m_saved.h\"\n".to_string(),
        ),
        (
            "m_sm.h".to_string(),
            "#include <stdbool.h>\n#include <string.h>\n#include \"gone.h\"\n\
             #include \"child_sm.h\"\n"
                .to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let reached = hosted_reached("m_sm.c", &files);
    let found = &reached.violations;
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("<string.h>") && found[0].contains("m_sm.c > m_sm.h > string.h"));
    assert!(found[1].contains("\"gone.h\" resolves to no file"));
    assert_eq!(reached.other_machines, ["child_sm.h"]);
}
