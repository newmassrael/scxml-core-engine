// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An unqualified attribute the SCXML Recommendation does not define for an
//! element is refused, in both pipelines and at the command line, rather than
//! read as something the author did not write.
//!
//! # What was wrong
//!
//! `schemas/sce-forge.xsd` takes a W3C element laxly and the parsers read the
//! attributes they know by name, so an attribute the Recommendation does not
//! define for an element was never asked for. Measured 2026-10-11, on a
//! document an AI wrote: `<state id="main" type="parallel">` built with exit
//! 0 from `check` and from `generate` and meant an ordinary compound state;
//! only its first region was entered and six of eight test cases failed.
//!
//! # What is held
//!
//! * The table `sce_build::scxml_attributes::SCXML_ATTRIBUTES` is the
//!   Recommendation's, element by element, read from the specification the
//!   ledger holds (`docs/spec/scxml/.atomic/epub`) and not from a copy kept
//!   here.
//! * Every document this repository commits carries none: the check refuses
//!   nothing the tree writes.
//! * The mistake that happened is refused where it is written, through both
//!   parsers and through the binary, with the element that was meant.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::parser::SCXMLParser;
use sce_build::scxml_attributes::{unknown_scxml_attributes, SCXML_ATTRIBUTES};
use sce_build::{DocumentLabel, Pipeline};

const CODE: &str = "\"validation/unknown-scxml-attribute\"";

/// Read the element-to-attribute map out of the specification's own text:
/// the first column of each element's "Attribute Details" table, in the order
/// the table lists it. An element with no such table maps to an empty list.
const SPEC_TABLES: &str = r#"
import zipfile, re, json, sys

s = zipfile.ZipFile(sys.argv[1]).read('OEBPS/spec.xhtml').decode('utf-8')
out = {}
current = None
for m in re.finditer(r'<h([2-4])[^>]*>(.*?)</h\1>', s, re.S):
    text = re.sub(r'\s+', ' ', re.sub(r'<[^>]*>', '', m.group(2))).strip()
    element = re.match(r'^\d+\.\d+ &lt;(\w+)&gt;$', text)
    if element:
        current = element.group(1)
        out.setdefault(current, [])
    elif re.match(r'^\d+\.\d+\.\d+ Attribute Details$', text) and current:
        rest = s[m.end():]
        nxt = re.search(r'<h[2-4][ >]', rest)
        section = rest[:nxt.start()] if nxt else rest
        cells = []
        for table in re.findall(r'<table>(.*?)</table>', section, re.S):
            cells += [re.sub(r'\s+', ' ', c).strip()
                      for c in re.findall(r'<tr>\s*<td>(.*?)</td>', table, re.S)]
        out[current] = cells
print(json.dumps(out))
"#;

fn epub() -> PathBuf {
    common::repository::root().join("docs/spec/scxml/.atomic/epub/scxml-REC-20150901.epub")
}

fn python_present() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `refusal` as the first record's code, row and message.
fn refusal_of(
    error: &sce_build::forge::error::Located<sce_build::forge::error::ForgeError>,
) -> (String, Option<u32>, String) {
    let record = error.error.to_diagnostics().remove(0);
    (
        serde_json::to_string(&record.code).expect("a code serialises"),
        error.location.line,
        record.message,
    )
}

#[test]
fn the_table_is_the_recommendations_attribute_table_for_every_element() {
    if !python_present() {
        assert!(
            std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
            "python3 is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
        );
        eprintln!("python3 absent: the table was not compared with the specification");
        return;
    }
    let out = Command::new("python3")
        .args(["-I", "-c", SPEC_TABLES])
        .arg(epub())
        .output()
        .expect("python3 runs the table extraction");
    assert!(
        out.status.success(),
        "the extraction failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut spec: BTreeMap<String, Vec<String>> =
        serde_json::from_slice(&out.stdout).expect("the extraction is a JSON object");
    assert!(
        spec.len() >= 25,
        "only {} element(s) read from the specification: {:?}",
        spec.len(),
        spec.keys().collect::<Vec<_>>()
    );

    // `xmlns` is a namespace declaration, which no parser sees as an
    // attribute, so the table leaves it out.
    for attributes in spec.values_mut() {
        attributes.retain(|a| a != "xmlns");
    }
    // §scxml-4.3: the specification states `cond` on `<elseif>` in the text
    // of `<if>` ("a tag whose 'cond' attribute") and in its example, not in a
    // table of `<elseif>`'s own.
    let epub_text = String::from_utf8_lossy(
        &Command::new("python3")
            .args([
                "-I",
                "-c",
                "import zipfile,sys;sys.stdout.write(zipfile.ZipFile(sys.argv[1]).read('OEBPS/spec.xhtml').decode())",
            ])
            .arg(epub())
            .output()
            .expect("python3 reads the specification")
            .stdout,
    )
    .into_owned();
    assert!(
        epub_text.contains("&lt;elseif cond="),
        "the specification no longer shows <elseif cond=…>; the exception below has lost its basis"
    );
    spec.insert("elseif".into(), vec!["cond".into()]);

    let mut wrong = Vec::new();
    for (element, attributes) in &spec {
        let held: Option<Vec<String>> = SCXML_ATTRIBUTES
            .iter()
            .find(|(name, _)| name == element)
            .map(|(_, held)| held.iter().map(|a| a.to_string()).collect());
        if held.as_ref() != Some(attributes) {
            wrong.push(format!(
                "  <{element}>: the specification lists {attributes:?}, the table holds {held:?}"
            ));
        }
    }
    for (element, _) in SCXML_ATTRIBUTES {
        if !spec.contains_key(*element) {
            wrong.push(format!(
                "  <{element}>: in the table, not in the specification"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "the table is not the specification's:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn no_document_this_tree_commits_carries_an_attribute_the_recommendation_does_not_define() {
    let documents = common::repository::files_git_tracks(&["*.scxml"]);
    assert!(
        documents.len() > 600,
        "found only {} committed SCXML documents; an empty sweep passes for the wrong reason",
        documents.len()
    );
    let (mut read, mut found) = (0usize, Vec::new());
    for path in &documents {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(document) = roxmltree::Document::parse(&text) else {
            continue;
        };
        read += 1;
        for unknown in unknown_scxml_attributes(&document.root_element()) {
            found.push(format!(
                "  {}:{}: <{}> {}",
                path.display(),
                unknown.row,
                unknown.element,
                unknown.attribute
            ));
        }
    }
    assert!(read > 600, "only {read} committed document(s) parse as XML");
    assert!(
        found.is_empty(),
        "the tree commits {} attribute(s) the Recommendation does not define:\n{}",
        found.len(),
        found.join("\n")
    );
}

const PARALLEL_AS_A_STATE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="main">
  <state id="main" type="parallel">
    <state id="a"/>
    <state id="b"/>
  </state>
</scxml>
"#;

#[test]
fn a_state_of_type_parallel_is_refused_where_it_is_written_with_the_element_that_means_it() {
    let outcome = SCXMLParser::new().parse_string(PARALLEL_AS_A_STATE, "parallel_as_a_state");
    let Err(refusal) = outcome else {
        panic!("the document is built, not refused");
    };
    let (code, row, message) = refusal_of(&refusal);
    assert_eq!((code.as_str(), row), (CODE, Some(3)), "{message}");
    assert!(
        message.contains("type is not an attribute of <state>") && message.contains("<parallel"),
        "{message}"
    );
}

/// A forge document is read by the other parser, which refuses the same.
#[test]
fn the_forge_parser_refuses_the_same() {
    let path = common::repository::root().join("tests/forge/resources/codec_cbor_map.scxml");
    let text = std::fs::read_to_string(&path).expect("the fixture reads");
    assert_eq!(sce_build::classify_document(&text), Pipeline::Forge);
    let at = text
        .find("<datamodel>")
        .expect("the fixture has a datamodel");
    let placed = format!(
        "{}<datamodel type=\"parallel\">{}",
        &text[..at],
        &text[at + "<datamodel>".len()..]
    );
    let row = text[..at].matches('\n').count() as u32 + 1;
    let outcome = sce_build::forge::parser::parse_forge_with_imports(
        &placed,
        DocumentLabel::symmetric("codec_cbor_map"),
    );
    let Err(refusal) = outcome else {
        panic!("the document is built, not refused");
    };
    let (code, line, message) = refusal_of(&refusal);
    assert_eq!((code.as_str(), line), (CODE, Some(row)), "{message}");
}

fn codegen(args: &[&str], document: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(args)
        .arg(document)
        .output()
        .expect("sce-codegen runs")
}

#[test]
fn the_command_line_exits_nonzero_for_check_and_for_generate() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("unknown-scxml-attribute-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    let document = dir.join("parallel_as_a_state.scxml");
    std::fs::write(&document, PARALLEL_AS_A_STATE).expect("the document is written");

    let check = codegen(&["check", "-l", "cpp"], &document);
    let generate = {
        let out = dir.join("out");
        std::fs::create_dir_all(&out).expect("output directory");
        Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
            .args(["generate"])
            .arg(&document)
            .args(["-l", "cpp", "-o"])
            .arg(&out)
            .output()
            .expect("sce-codegen runs")
    };
    let _ = std::fs::remove_dir_all(&dir);

    for (what, output) in [("check", &check), ("generate", &generate)] {
        let streams = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "{what} accepted a <state type=\"parallel\">: {streams}"
        );
        assert!(
            streams.contains("unknown-scxml-attribute")
                || streams.contains("not an attribute of <state>"),
            "{what} refused it for another reason: {streams}"
        );
    }
}
