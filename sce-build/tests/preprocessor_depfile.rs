// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Pins the contract that `SCXMLParser::parse_file` records every
// external file it consumed during preprocessor expansion (`<sce:use>`
// templates and `<xi:include>` fragments) into
// `SCXMLParser::preprocessor_deps()`. The codegen CLI surfaces this
// list through `--write-deps`; without it CMake/Ninja would treat
// fragment edits as silent no-ops and ship stale generated state
// machines.
//
// Three invariants — one per call shape:
//   1. No external inputs           → empty deps (current behaviour, regression guard).
//   2. Single `<sce:use template>`  → one dep entry pointing at the resolved fragment.
//   3. Transitive `<sce:use>` chain → every visited fragment in deps, in DFS order.
//
// Originating ticket: tc8-harness feedback report on
// `sce-codegen --write-deps` omitting user-side fragments.

use sce_build::parser::SCXMLParser;
use std::fs;
use tempfile::tempdir;

const SCE_NS: &str = "http://sce.dev/ext";

/// Helper: write `name` under `dir` with `body`. Returns the absolute
/// canonical path so tests can compare exactly what
/// `preprocessor_deps()` emitted.
fn write_file(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).expect("write fixture file");
    fs::canonicalize(&path).expect("canonicalize fixture path")
}

#[test]
fn preprocessor_deps_empty_for_pure_scxml() {
    let tmp = tempdir().expect("tempdir");
    let host = write_file(
        tmp.path(),
        "host.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="host" initial="s">
  <state id="s">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
    );

    let mut parser = SCXMLParser::new();
    parser
        .parse_file(host.to_str().unwrap())
        .expect("plain SCXML parses");

    assert!(
        parser.preprocessor_deps().is_empty(),
        "documents without <sce:use> or <xi:include> must produce no deps; \
         got {:?}",
        parser.preprocessor_deps()
    );
}

#[test]
fn preprocessor_deps_records_sce_use_fragment() {
    let tmp = tempdir().expect("tempdir");
    let fragment = write_file(
        tmp.path(),
        "shared.sce-template.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml"
              xmlns:sce="http://sce.dev/ext"
              name="shared">
  <state id="phase1">
    <transition event="tick" target="pass"/>
  </state>
  <final id="pass"/>
</sce:template>
"#,
    );
    let host = write_file(
        tmp.path(),
        "host.scxml",
        &format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="{SCE_NS}"
       version="1.0" name="host" initial="phase1">
  <sce:use template="shared.sce-template.xml"/>
</scxml>
"#
        ),
    );

    let mut parser = SCXMLParser::new();
    parser
        .parse_file(host.to_str().unwrap())
        .expect("host with <sce:use> parses");

    let deps = parser.preprocessor_deps();
    assert_eq!(
        deps.len(),
        1,
        "exactly one fragment was opened; got {:?}",
        deps
    );
    assert_eq!(
        deps[0], fragment,
        "deps[0] must match the resolved fragment path"
    );
}

#[test]
fn preprocessor_deps_records_transitive_sce_use_chain() {
    // host → mid → leaf. The expander opens both fragments while
    // resolving the chain, so both must surface in deps.
    let tmp = tempdir().expect("tempdir");
    let leaf = write_file(
        tmp.path(),
        "leaf.sce-template.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml"
              xmlns:sce="http://sce.dev/ext"
              name="leaf">
  <state id="phase2">
    <transition event="tick" target="pass"/>
  </state>
  <final id="pass"/>
</sce:template>
"#,
    );
    let mid = write_file(
        tmp.path(),
        "mid.sce-template.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml"
              xmlns:sce="http://sce.dev/ext"
              name="mid">
  <sce:use template="leaf.sce-template.xml"/>
</sce:template>
"#,
    );
    let host = write_file(
        tmp.path(),
        "host.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="host" initial="phase2">
  <sce:use template="mid.sce-template.xml"/>
</scxml>
"#,
    );

    let mut parser = SCXMLParser::new();
    parser
        .parse_file(host.to_str().unwrap())
        .expect("host with transitive <sce:use> parses");

    let deps = parser.preprocessor_deps();
    assert!(
        deps.contains(&mid),
        "deps must include the directly-used fragment; got {:?}",
        deps
    );
    assert!(
        deps.contains(&leaf),
        "deps must include the transitively-used fragment; got {:?}",
        deps
    );
    assert_eq!(
        deps.len(),
        2,
        "exactly two fragments visited (mid + leaf); got {:?}",
        deps
    );
    // DFS order: `mid` is opened before its body resolves `leaf`,
    // so deps[0] == mid, deps[1] == leaf. Pinning order helps the
    // depfile-consumer side (build systems are insensitive but the
    // reproducibility contract is cleaner with a fixed shape).
    assert_eq!(deps[0], mid, "DFS order: directly-used fragment first");
    assert_eq!(deps[1], leaf, "DFS order: transitively-used fragment second");
}

#[test]
fn preprocessor_deps_cleared_between_parses() {
    // Re-using a parser instance must not leak deps from a previous
    // file. parse_file starts by clearing the field; this regression
    // guard pins that contract — without the clear, a downstream
    // consumer that retains a parser across multiple files would
    // emit stale fragment paths in later depfiles.
    let tmp = tempdir().expect("tempdir");
    let fragment = write_file(
        tmp.path(),
        "f.sce-template.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml"
              xmlns:sce="http://sce.dev/ext"
              name="f">
  <final id="done"/>
</sce:template>
"#,
    );
    let with_use = write_file(
        tmp.path(),
        "with_use.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="with_use" initial="done">
  <sce:use template="f.sce-template.xml"/>
</scxml>
"#,
    );
    let plain = write_file(
        tmp.path(),
        "plain.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="plain" initial="done">
  <final id="done"/>
</scxml>
"#,
    );

    let mut parser = SCXMLParser::new();
    parser.parse_file(with_use.to_str().unwrap()).unwrap();
    assert_eq!(parser.preprocessor_deps(), &[fragment]);

    parser.parse_file(plain.to_str().unwrap()).unwrap();
    assert!(
        parser.preprocessor_deps().is_empty(),
        "second parse with no preprocessor inputs must clear the list; \
         got {:?}",
        parser.preprocessor_deps()
    );
}
