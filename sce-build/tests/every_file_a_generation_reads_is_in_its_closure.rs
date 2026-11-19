// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The files a generation reads, as the parser and the loader record them,
//! are the inputs a generated file's `source-hash` must cover (ADR 0094 section
//! 1). This file holds the recorded reads complete where a read is known to
//! happen.

use sce_build::forge::drift::{ClosureOptions, InputClosure};
use sce_build::parser::SCXMLParser;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The directories the sweep reads, relative to the repository: the fixtures
/// of the committed `static_datamodel` trees, the test profile's and the
/// unions', the integration fixtures, the W3C resources and the template
/// parity fixtures. Between them they hold every kind of reference a closure
/// follows, which the sweep asserts it met. Every XInclude statechart among
/// them is refused on purpose (a diagnostic fixture), so `input_closure`
/// holds the one whose fragment is spliced.
const SWEPT: [&str; 7] = [
    "sce-build/tests/fixtures/static_datamodel",
    "sce-build/tests/fixtures/test_profile",
    "sce-build/tests/fixtures/union",
    "sce-build/tests/fixtures/input_closure",
    "integration_resources",
    "resources",
    "tests/w3c_template_parity/fixtures",
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits in the repository")
        .to_path_buf()
}

/// Every `.scxml` under `dir`, recursively, sorted.
fn documents_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            documents_under(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "scxml")
        {
            out.push(path);
        }
    }
}

/// Which kinds of reference a statechart's text makes, as a closure follows
/// them: `[src child, candidate, import, include]`.
fn reference_kinds(text: &str) -> [bool; 4] {
    let Ok(document) = roxmltree::Document::parse(text) else {
        return [false; 4];
    };
    let mut kinds = [false; 4];
    for node in document.descendants().filter(|node| node.is_element()) {
        match node.tag_name().name() {
            "invoke" => {
                let src = node.attribute("src").unwrap_or("");
                kinds[0] |= src.ends_with(".scxml")
                    && !src.starts_with("sce:std/")
                    && node.attribute("srcexpr").is_none();
                kinds[1] |= node
                    .attributes()
                    .any(|attribute| attribute.name() == "candidates");
            }
            "import" => kinds[2] = true,
            "include" => kinds[3] = true,
            _ => {}
        }
    }
    kinds
}

/// ADR 0094 test 3, the soundness sweep: for every statechart of the swept
/// directories that parses, every file its parse recorded reading (the
/// fragments, the children it read for their interface, the candidates)
/// and every document its `<sce:import>`s name is a member of its input
/// closure. A member missing from the closure is an input whose edit would
/// change the output and leave the header as it was. The two lists are the
/// acceptance record's inputs for a statechart, so the record is held inside
/// the closure too.
#[test]
fn every_file_a_statechart_reads_is_in_its_closure() {
    let root = repository();
    let mut documents = Vec::new();
    for dir in SWEPT {
        documents_under(&root.join(dir), &mut documents);
    }
    let mut met = [0usize; 4];
    let mut swept = 0usize;
    let mut outside: Vec<String> = Vec::new();
    for document in &documents {
        let Ok(text) = fs::read_to_string(document) else {
            continue;
        };
        if !matches!(
            sce_build::classify_document(&text),
            sce_build::Pipeline::Scxml
        ) {
            continue;
        }
        let mut parser = SCXMLParser::new();
        let Ok(model) = parser.parse_file(&document.display().to_string()) else {
            continue;
        };
        let base = document.parent().unwrap_or_else(|| Path::new("."));
        let mut recorded: Vec<PathBuf> = parser.preprocessor_deps().to_vec();
        recorded.extend(
            model
                .forge_imports
                .iter()
                .map(|import| Path::new(&import.src))
                .filter(|src| !sce_build::forge::stdlib::names_standard(src))
                .map(|src| base.join(src)),
        );
        let closure = InputClosure::of(&[document.as_path()], ClosureOptions::default())
            .unwrap_or_else(|e| panic!("the closure of {} is read: {e}", document.display()));
        let members: BTreeSet<PathBuf> = closure.contributing_paths().into_iter().collect();
        for read in &recorded {
            let Ok(real) = fs::canonicalize(read) else {
                continue;
            };
            if !members.contains(&real) {
                outside.push(format!("{} reads {}", document.display(), real.display()));
            }
        }
        swept += 1;
        if !recorded.is_empty() {
            for (count, kind) in met.iter_mut().zip(reference_kinds(&text)) {
                *count += usize::from(kind);
            }
        }
    }
    assert!(
        outside.is_empty(),
        "{} read(s) of {swept} statechart(s) are outside their closure:\n{}",
        outside.len(),
        outside.join("\n")
    );
    assert!(
        swept >= 100,
        "the sweep read only {swept} statechart(s); the swept directories moved"
    );
    let names = ["a src child", "a candidate", "an import", "an include"];
    for (count, name) in met.iter().zip(names) {
        assert!(
            *count > 0,
            "no swept statechart with recorded reads has {name}, so the sweep \
             proves nothing about that kind of reference"
        );
    }
}

/// `path` as the file system resolves it, the spelling the recorded reads use.
fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|e| panic!("{} resolves: {e}", path.display()))
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).expect("write a document");
    path
}

const PARENT_STARTING_A_FILE_CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="run" name="parent">
  <state id="run">
    <invoke src="child.scxml"/>
  </state>
</scxml>
"#;

const CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done" name="child">
  <final id="done"/>
</scxml>
"#;

/// A statechart reads the document its `<invoke src>` starts to learn the
/// child's interface (`parse_child_metadata`), as it reads a candidate. Until
/// ADR 0094 the candidate was recorded and the `src` child was not, so a
/// depfile did not declare it and an edit to the child left the parent's
/// output stale.
#[test]
fn a_static_src_child_is_a_recorded_read_of_the_document_that_starts_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let parent = write(dir.path(), "parent.scxml", PARENT_STARTING_A_FILE_CHILD);
    let child = write(dir.path(), "child.scxml", CHILD);

    let mut parser = SCXMLParser::new();
    parser
        .parse_file(&parent.display().to_string())
        .unwrap_or_else(|e| panic!("the parent parses: {}", e.error));
    let reads: Vec<PathBuf> = parser
        .preprocessor_deps()
        .iter()
        .map(|read| canonical(read.as_path()))
        .collect();
    assert!(
        reads.contains(&canonical(&child)),
        "the child the parent starts by `src` is one of its recorded reads: {reads:?}"
    );
}
