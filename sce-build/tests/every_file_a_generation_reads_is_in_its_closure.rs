// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The files a generation reads, as the parser and the loader record them,
//! are the inputs a generated file's `source-hash` must cover (ADR 0094 section
//! 1). This file holds the recorded reads complete where a read is known to
//! happen.

use sce_build::parser::SCXMLParser;
use std::fs;
use std::path::{Path, PathBuf};

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
