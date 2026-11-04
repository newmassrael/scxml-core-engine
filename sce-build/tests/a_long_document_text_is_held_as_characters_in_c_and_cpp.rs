// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document text too long for one string literal of the target is written as
//! the characters of its bytes, at every kind of site the C11 and C++
//! templates write one.
//!
//! ISO C11 (5.2.4.1) promises 4095 characters in a string literal, counted
//! after adjacent literals are joined, and GCC's `-Woverlength-strings`, which
//! `-Wpedantic` turns on and `-Werror` makes fatal, refuses a longer one. MSVC
//! holds no C++ literal past 65,535 bytes, and Clang's `-Woverlength-strings`
//! refuses one past 65,536 characters. So a text of 4095 bytes is a C literal
//! and one of 4096 is not, and the same at 65,535 and 65,536 for C++.
//!
//! One document carries a text at each kind of site, each spelled with its
//! own letter so the generated source says which site a text came from:
//!
//! * a `<log label>`, handed to a call (C: `fprintf`, a compound literal past
//!   the limit; C++: `SCE_LOG_INFO`, a `std::string`);
//! * the inline XML `<content>` of a `<send>` (C: an array of static storage;
//!   C++: the `std::string` the content lands in);
//! * the static `<param>` of a host-served `<send>` (C: the array of static
//!   storage the host's pointer is kept to; C++: the string pushed into the
//!   request's map);
//! * on C, a `<data>` element's inline XML, handed to the DOM reader.
//!
//! Nothing here compiles the output: the claim is that no literal of the
//! target holds more than the limit, which is what the compilers refuse, and
//! that the text is there whole as characters.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::template_lexing::{classify, Class, Syntax};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// A scratch directory of this test's own, emptied first.
fn scratch(tag: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("sce-long-text-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

/// The document, with the text each site carries.
fn document(label: &str, data: &str, content: &str, param: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" name="long_text" initial="s">
  <datamodel><data id="d">{data}</data></datamodel>
  <state id="s">
    <onentry>
      <log label="{label}"/>
      <send event="e" target="#_internal"><content>{content}</content></send>
      <send event="h" type="x-sce-host"><param name="p" expr="'{param}'"/></send>
    </onentry>
  </state>
</scxml>
"##
    )
}

/// Every file `sce-codegen` writes for `document` in `language`, concatenated.
fn generate(document: &str, language: &str, tag: &str) -> String {
    let dir = scratch(tag);
    let source = dir.join("long_text.scxml");
    std::fs::write(&source, document).expect("the document is written");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("output directory");
    let run = Command::new(codegen_bin())
        .args([
            "generate",
            source.to_str().expect("UTF-8 path"),
            "-l",
            language,
            "-o",
            out.to_str().expect("UTF-8 path"),
            "--host-processor",
            "x-sce-host",
        ])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert!(
        run.status.success(),
        "{language} refused the document: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut files: Vec<PathBuf> = std::fs::read_dir(&out)
        .expect("output directory")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    files.sort();
    files
        .iter()
        .map(|path| std::fs::read_to_string(path).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The bodies of the escapable string literals of a C or C++ source, quotes
/// stripped, as the template lexer reads the source (comments and character
/// constants are not literals).
fn literal_bodies(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let classes = classify(source, Syntax::CFamily);
    let mut bodies = Vec::new();
    let mut at = 0usize;
    while at < classes.len() {
        if classes[at] != Class::Literal {
            at += 1;
            continue;
        }
        let start = at;
        while at < classes.len() && classes[at] == Class::Literal {
            at += 1;
        }
        let span: String = chars[start..at].iter().collect();
        bodies.push(
            span.strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&span)
                .to_string(),
        );
    }
    bodies
}

/// `<r>` around `n` copies of `letter`: an XML text of `n + 7` bytes.
fn xml(letter: char, n: usize) -> String {
    format!("<r>{}</r>", letter.to_string().repeat(n))
}

/// The character constants `text` starts with, as the door writes them.
fn constants(text: &str, n: usize) -> String {
    text.chars()
        .take(n)
        .map(|c| format!("'{c}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A text of exactly `limit` bytes is one literal; one of `limit + 1` is in no
/// literal, and `starts[i]` (the spelling its kind of site writes the
/// characters in) is in the source.
fn holds(language: &str, limit: usize, sites: &[(char, bool)], starts: &[&str], data: bool) {
    let text_of = |letter: char, xml_text: bool, n: usize| -> String {
        if xml_text {
            xml(letter, n - 7)
        } else {
            letter.to_string().repeat(n)
        }
    };
    let short = xml('d', 1);
    let build = |n: usize| {
        let mut texts = sites.iter().map(|&(letter, x)| text_of(letter, x, n));
        let label = texts.next().expect("a label");
        let content = texts.next().expect("a content");
        let param = texts.next().expect("a param");
        let data_text = if data {
            texts.next().expect("a data")
        } else {
            short.clone()
        };
        (label, data_text, content, param)
    };

    let (label, data_text, content, param) = build(limit);
    let source = generate(
        &document(&label, &data_text, &content, &param),
        language,
        &format!("{language}-fits"),
    );
    let bodies = literal_bodies(&source);
    for text in [&label, &content, &param]
        .into_iter()
        .chain(data.then_some(&data_text))
    {
        assert_eq!(text.len(), limit);
        assert!(
            bodies.iter().any(|body| body == text),
            "{language}: a text of {limit} bytes ({}...) is not one string literal",
            &text[..8]
        );
    }

    let (label, data_text, content, param) = build(limit + 1);
    let source = generate(
        &document(&label, &data_text, &content, &param),
        language,
        &format!("{language}-over"),
    );
    let bodies = literal_bodies(&source);
    let texts: Vec<&String> = [&label, &content, &param]
        .into_iter()
        .chain(data.then_some(&data_text))
        .collect();
    for (text, start) in texts.iter().zip(starts) {
        assert_eq!(text.len(), limit + 1);
        let longest = bodies.iter().map(String::len).max().unwrap_or(0);
        assert!(
            !bodies.iter().any(|body| body.contains(text.as_str())),
            "{language}: a text of {} bytes ({}...) is still a string literal (the \
             longest literal holds {longest} bytes)",
            limit + 1,
            &text[..8]
        );
        let spelled = format!("{start}{}", constants(text, 4));
        assert!(
            source.contains(&spelled),
            "{language}: a text of {} bytes is not written as `{spelled}...`",
            limit + 1
        );
    }
}

#[test]
fn a_c_text_of_4095_bytes_is_a_literal_and_one_of_4096_is_its_characters() {
    holds(
        "c11",
        4095,
        &[('L', false), ('C', true), ('P', false), ('D', true)],
        &[
            // A call's argument: a compound literal, parenthesized for a macro.
            "((const char[]){",
            // The send's inline XML: the array of static storage it is copied from.
            "[] = {",
            // The host's parameter: the array of static storage its pointer is kept to.
            "[] = {",
            // The data's inline XML, handed to the DOM reader.
            "((const char[]){",
        ],
        true,
    );
}

#[test]
fn a_cpp_text_of_65535_bytes_is_a_literal_and_one_of_65536_is_its_characters() {
    holds(
        "cpp",
        65_535,
        &[('L', false), ('C', true), ('P', false)],
        &["(std::string{", "(std::string{", "(std::string{"],
        false,
    );
}
