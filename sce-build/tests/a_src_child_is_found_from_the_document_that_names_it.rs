// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A child a document starts by `src` is the file `src` names from that
//! document, at every depth, and a run stages each one, with the children it
//! starts by `src` in turn, into its output directory, where the generation of
//! each staged child reads it (§scxml-6.4).
//!
//! # What was wrong
//!
//! A run staged the children its own machines start, each looked for beside
//! the document under the name of its machine, whatever directory `src` named.
//! A staged child's generation then read the copy in the output directory and
//! looked for its own `src` child beside that copy, where nothing had put it:
//! the generation, the parse of the child's machine and the plan of it
//! (`--plan-children`) all found no document, so the machine the child's
//! machine names had no generation and no rule.
//!
//! # What is held
//!
//! A parent starts a child by `src="file:sub/nested_src_child.scxml"`, and the
//! child starts its own by `src="file:nested_src_grandchild.scxml"`, beside
//! itself in `sub/`; the documents are written here, so the test reads no
//! tracked file. For C++ and C11:
//!
//! * the plan of the parent (`--plan --plan-children`, as the CMake routes ask
//!   it) names both documents among the files the parent's run writes, plans
//!   the generation of each with the arguments it states (`--as-child`), and
//!   is one `cmake/SCECodegenPlan.cmake` accepts: every child document the run
//!   writes has a planned generation, each generation reads a document the
//!   run or a generation before it writes, and each names its machine;
//! * the parent's run, then each planned generation in the plan's order, run
//!   with the arguments the plan states, adds exactly the files planned for
//!   it, the grandchild's machine among them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running"
       name="nested_src">
    <state id="running">
        <invoke type="http://www.w3.org/TR/scxml/" src="file:sub/nested_src_child.scxml"/>
    </state>
</scxml>
"#;

/// Written in `sub/`, beside the document it starts.
const CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting"
       name="nested_src_child">
    <state id="waiting">
        <invoke type="http://www.w3.org/TR/scxml/" src="file:nested_src_grandchild.scxml"/>
    </state>
</scxml>
"#;

const GRANDCHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle"
       name="nested_src_grandchild">
    <state id="idle"/>
</scxml>
"#;

const LANGUAGES: &[&str] = &["cpp", "c11"];

/// The three documents written where their author keeps them; the directory
/// (kept alive) and the parent's path.
fn authored() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("authored");
    std::fs::create_dir_all(root.join("sub")).expect("make the documents' directories");
    std::fs::write(root.join("nested_src.scxml"), PARENT).expect("write the parent");
    std::fs::write(root.join("sub/nested_src_child.scxml"), CHILD).expect("write the child");
    std::fs::write(root.join("sub/nested_src_grandchild.scxml"), GRANDCHILD)
        .expect("write the grandchild");
    (dir, root.join("nested_src.scxml"))
}

/// `sce-codegen [flags] generate <document> [arguments] -l <language> -o <out>
/// --input-root <the parent's directory>`, as the CMake routes run every
/// generation of a fixture.
fn run(
    flags: &[&str],
    document: &Path,
    arguments: &[String],
    language: &str,
    out: &Path,
    root: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(flags)
        .arg("generate")
        .arg(document)
        .args(arguments)
        .args(["-l", language, "--no-format", "-o"])
        .arg(out)
        .arg("--input-root")
        .arg(root)
        .output()
        .expect("spawn sce-codegen")
}

fn succeeded(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// One generation a plan holds: the document it reads, the arguments it is
/// run with, and the files it writes.
struct Generation {
    document: String,
    arguments: Vec<String>,
    files: BTreeSet<String>,
}

/// A `--plan-children` answer read as `sce_codegen_plan` reads it: the run's
/// own files, then each generation in the order the generator planned it.
fn read_plan(output: &Output) -> (BTreeSet<String>, Vec<Generation>) {
    let mut own = BTreeSet::new();
    let mut generations: Vec<Generation> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((document, rest)) = line.split_once('\t') else {
            own.insert(line.to_string());
            continue;
        };
        if let Some(stated) = rest.strip_prefix('\t') {
            assert!(
                generations
                    .iter()
                    .all(|planned| planned.document != document),
                "the arguments of {document}'s generation are stated twice"
            );
            generations.push(Generation {
                document: document.to_string(),
                arguments: stated.split('\t').map(str::to_string).collect(),
                files: BTreeSet::new(),
            });
            continue;
        }
        let generation = generations
            .iter_mut()
            .find(|planned| planned.document == document)
            .unwrap_or_else(|| panic!("a file of {document}'s generation before its arguments"));
        generation.files.insert(rest.to_string());
    }
    (own, generations)
}

/// The stem of the document at `path`.
fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .expect("a document name")
        .to_string_lossy()
        .into_owned()
}

/// What `cmake/SCECodegenPlan.cmake` refuses in a plan asked with
/// `CHILD_GENERATIONS`, read off the same answer: a child document the run
/// writes with no planned generation, a generation of a document neither the
/// run nor a generation before it writes, a generation that does not name its
/// machine. Empty is a plan the build accepts.
fn plan_refusals(
    own: &BTreeSet<String>,
    generations: &[Generation],
    out: &Path,
    language: &str,
) -> Vec<String> {
    let mut refusals = Vec::new();
    let machine = |stem: &str| -> Vec<String> {
        let header = out.join(format!("{stem}_sm.h")).display().to_string();
        if language == "c11" {
            vec![
                header,
                out.join(format!("{stem}_sm.c")).display().to_string(),
            ]
        } else {
            vec![header]
        }
    };
    for document in own.iter().filter(|path| path.ends_with(".scxml")) {
        if document.contains("__sce_synth_invoke__") {
            continue;
        }
        if !generations
            .iter()
            .any(|planned| &planned.document == document)
        {
            refusals.push(format!(
                "{document} is written and no generation of it is planned"
            ));
        }
    }
    let mut written: BTreeSet<&String> = own.iter().collect();
    for generation in generations {
        if !written.contains(&generation.document) {
            refusals.push(format!(
                "{} is generated and neither the run nor a generation before it writes it",
                generation.document
            ));
        }
        for file in machine(&stem(&generation.document)) {
            if !generation.files.contains(&file) {
                refusals.push(format!(
                    "the generation of {} does not name its machine {file}",
                    generation.document
                ));
            }
        }
        written.extend(generation.files.iter());
    }
    refusals
}

/// The paths of the files under `dir`, at any depth.
fn files_under(dir: &Path) -> BTreeSet<String> {
    fn walk(dir: &Path, files: &mut BTreeSet<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, files);
            } else {
                files.insert(path.display().to_string());
            }
        }
    }
    let mut files = BTreeSet::new();
    walk(dir, &mut files);
    files
}

#[test]
fn the_plan_of_a_parent_plans_the_generation_of_its_src_childs_src_child() {
    for language in LANGUAGES {
        let (dir, parent) = authored();
        let root = parent.parent().expect("the parent's directory");
        let out = dir.path().join("out");
        let plan = run(
            &["--plan", "--plan-children"],
            &parent,
            &[],
            language,
            &out,
            root,
        );
        succeeded(&plan, &format!("[{language}] the plan of the parent"));
        let (own, generations) = read_plan(&plan);

        let child = out.join("nested_src_child.scxml").display().to_string();
        let grandchild = out
            .join("nested_src_grandchild.scxml")
            .display()
            .to_string();
        for document in [&child, &grandchild] {
            assert!(
                own.contains(document),
                "[{language}] the parent's run does not stage {document}: {own:?}"
            );
        }
        for document in [&child, &grandchild] {
            let generation = generations
                .iter()
                .find(|planned| &planned.document == document)
                .unwrap_or_else(|| {
                    panic!("[{language}] the plan holds no generation of {document}")
                });
            assert_eq!(
                generation.arguments,
                ["--as-child"],
                "[{language}] the generation of {document} is not one in its parent's process"
            );
        }
        let refusals = plan_refusals(&own, &generations, &out, language);
        assert!(
            refusals.is_empty(),
            "[{language}] cmake/SCECodegenPlan.cmake would refuse the plan:\n  {}",
            refusals.join("\n  ")
        );
        assert!(
            !out.exists(),
            "[{language}] the plan wrote the output directory"
        );
    }
}

#[test]
fn each_planned_generation_writes_what_its_plan_names() {
    for language in LANGUAGES {
        let (dir, parent) = authored();
        let root = parent.parent().expect("the parent's directory");
        let planned_into = dir.path().join("planned");
        let written_into = dir.path().join("written");
        let plan = run(
            &["--plan", "--plan-children"],
            &parent,
            &[],
            language,
            &planned_into,
            root,
        );
        succeeded(&plan, &format!("[{language}] the plan of the parent"));
        let (own, generations) = read_plan(&plan);
        // The plan's paths, moved to the directory the runs write.
        let moved = |files: &BTreeSet<String>| -> BTreeSet<String> {
            files
                .iter()
                .map(|file| {
                    let name = Path::new(file)
                        .strip_prefix(&planned_into)
                        .unwrap_or_else(|_| panic!("{file} is not in the output directory"));
                    written_into.join(name).display().to_string()
                })
                .collect()
        };

        let parent_run = run(&[], &parent, &[], language, &written_into, root);
        succeeded(&parent_run, &format!("[{language}] the parent's run"));
        let after_parent = files_under(&written_into);
        assert_eq!(
            after_parent,
            moved(&own),
            "[{language}] the parent's run wrote other files than its plan named"
        );

        for generation in &generations {
            let document = moved(&BTreeSet::from([generation.document.clone()]))
                .pop_first()
                .expect("one document");
            let before = files_under(&written_into);
            let child_run = run(
                &[],
                Path::new(&document),
                &generation.arguments,
                language,
                &written_into,
                root,
            );
            succeeded(
                &child_run,
                &format!("[{language}] the generation of {document}"),
            );
            let added: BTreeSet<String> = files_under(&written_into)
                .difference(&before)
                .cloned()
                .collect();
            assert_eq!(
                added,
                moved(&generation.files),
                "[{language}] the generation of {document} added other files than its plan \
                 named"
            );
        }
        assert!(
            written_into.join("nested_src_grandchild_sm.h").is_file(),
            "[{language}] no generation wrote the machine of the src child's src child"
        );
    }
}
