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
//! Two readings make the claim the compilers' and not the template's:
//!
//! * The text a site carries is the one the parser holds, which for an XML
//!   element is its serialization with the namespace it inherits
//!   (`xmlns="http://www.w3.org/2005/07/scxml"`), not the characters written
//!   between the tags; each text is padded until the model holds exactly the
//!   limit.
//! * A literal is what the compiler's concatenation of adjacent literals makes. The
//!   C++ output passes through clang-format, whose `BreakStringLiterals`
//!   splits a long literal into adjacent pieces at the column limit; the
//!   compiler joins them back, and its limit counts the joined literal. A
//!   literal's value is read through its escapes (`\"`, octal), since that is
//!   what the compiler counts.
//!
//! Nothing here compiles the output: the claim is that no literal of the
//! target holds more than the limit, which is what the compilers refuse, and
//! that the text is there whole as characters.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::parser::SCXMLParser;
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

/// How many copies of its letter each site carries.
#[derive(Clone, Copy)]
struct Counts {
    label: usize,
    content: usize,
    param: usize,
    data: usize,
}

/// The document, each site carrying `counts` copies of its letter.
fn document(counts: Counts) -> String {
    let run = |letter: &str, n: usize| letter.repeat(n);
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" name="long_text" initial="s">
  <datamodel><data id="d"><r>{data}</r></data></datamodel>
  <state id="s">
    <onentry>
      <log label="{label}"/>
      <send event="e" target="#_internal"><content><r>{content}</r></content></send>
      <send event="h" type="x-sce-host"><param name="p" expr="'{param}'"/></send>
    </onentry>
  </state>
</scxml>
"##,
        data = run("D", counts.data),
        label = run("L", counts.label),
        content = run("C", counts.content),
        param = run("P", counts.param),
    )
}

/// The text the parser holds at each site of `document`: label, content,
/// param, data, in that order.
fn held(document: &str) -> [String; 4] {
    let model = SCXMLParser::new()
        .parse_string(document, "long_text")
        .unwrap_or_else(|e| panic!("the document parses: {e}"));
    let block = &model.states["s"].on_entry_blocks[0];
    [
        block[0].label.clone(),
        block[1].content.clone(),
        block[2].params[0].static_value.clone(),
        model.variables[0].content.clone(),
    ]
}

/// The counts at which each site's held text is exactly `bytes` long: what a
/// site adds around its letters is measured, not assumed.
fn counts_for(bytes: usize) -> Counts {
    let probe = held(&document(Counts {
        label: 1,
        content: 1,
        param: 1,
        data: 1,
    }));
    let pad = |held: &str| {
        assert!(!held.is_empty(), "a site holds the letter it was given");
        bytes - (held.len() - 1)
    };
    Counts {
        label: pad(&probe[0]),
        content: pad(&probe[1]),
        param: pad(&probe[2]),
        data: pad(&probe[3]),
    }
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

/// The value of a C or C++ literal body: the bytes its escapes denote.
fn decode(body: &str) -> Vec<u8> {
    let bytes = body.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' || i + 1 == bytes.len() {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let escape = bytes[i + 1];
        i += 2;
        match escape {
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'0'..=b'7' => {
                let mut value = u32::from(escape - b'0');
                for _ in 0..2 {
                    match bytes.get(i).copied() {
                        Some(digit @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(digit - b'0');
                            i += 1;
                        }
                        _ => break,
                    }
                }
                out.push(u8::try_from(value).expect("an octal escape names a byte"));
            }
            other => out.push(other),
        }
    }
    out
}

/// The values of the string literals of a C or C++ source, as the compiler
/// joins them: adjacent literals, with only white space between them, are one.
/// Comments and character constants are not literals (the template lexer's
/// reading).
fn literal_values(source: &str) -> Vec<Vec<u8>> {
    let chars: Vec<char> = source.chars().collect();
    let classes = classify(source, Syntax::CFamily);
    let mut values: Vec<Vec<u8>> = Vec::new();
    let mut joining = false;
    let mut at = 0usize;
    while at < classes.len() {
        if classes[at] != Class::Literal {
            if !chars[at].is_whitespace() {
                joining = false;
            }
            at += 1;
            continue;
        }
        let start = at;
        while at < classes.len() && classes[at] == Class::Literal {
            at += 1;
        }
        let span: String = chars[start..at].iter().collect();
        let body = span
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(&span);
        let value = decode(body);
        match values.last_mut() {
            Some(last) if joining => last.extend(value),
            _ => values.push(value),
        }
        joining = true;
    }
    values
}

/// `source` with the white space between its tokens removed, so a list of
/// characters reads the same however a formatter wrapped it. A string literal
/// and a character constant (`' '` is one) keep theirs.
fn compact(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let classes = classify(source, Syntax::CFamily);
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if classes[i] == Class::Literal {
            out.push(c);
            i += 1;
        } else if c == '\'' && classes[i] == Class::Code {
            // A character constant, copied whole, its escape included.
            out.push(c);
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    out.push(chars[i]);
                    i += 1;
                }
                out.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                out.push(chars[i]);
                i += 1;
            }
        } else {
            if !c.is_whitespace() {
                out.push(c);
            }
            i += 1;
        }
    }
    out
}

/// The character constants the first `n` bytes of `text` are written as, by
/// the door's own spelling of them, joined as [`compact`] leaves a list.
fn constants(text: &str, n: usize) -> String {
    sce_build::filters::c_char_constants(&text[..n]).join(",")
}

/// A text of exactly `limit` bytes is one literal; one of `limit + 1` is in no
/// literal, and `starts[i]` (the spelling its kind of site writes the
/// characters in) is in the source. `sites` says which of label, content,
/// param and data the language is held to.
fn holds(language: &str, limit: usize, sites: &[usize], starts: &[&str]) {
    // A site the language is not held to carries one letter.
    let counts = |bytes: usize| {
        let mut counts = counts_for(bytes);
        if !sites.contains(&DATA) {
            counts.data = 1;
        }
        counts
    };
    let fits = document(counts(limit));
    let texts = held(&fits);
    let source = generate(&fits, language, &format!("{language}-fits"));
    let values = literal_values(&source);
    for &site in sites {
        let text = &texts[site];
        assert_eq!(
            text.len(),
            limit,
            "the padding measures what the site holds"
        );
        assert!(
            values.iter().any(|value| value == text.as_bytes()),
            "{language}: a text of {limit} bytes ({}...) is not one string literal",
            &text[..12]
        );
    }

    let over = document(counts(limit + 1));
    let texts = held(&over);
    let source = generate(&over, language, &format!("{language}-over"));
    let values = literal_values(&source);
    let compacted = compact(&source);
    let longest = values.iter().map(Vec::len).max().unwrap_or(0);
    assert!(
        longest <= limit,
        "{language}: a literal of {longest} bytes stands in the source, past the {limit} \
         a literal holds"
    );
    for (&site, start) in sites.iter().zip(starts) {
        let text = &texts[site];
        assert_eq!(text.len(), limit + 1);
        let spelled = format!("{start}{}", constants(text, 4));
        assert!(
            compacted.contains(&spelled),
            "{language}: a text of {} bytes ({}...) is not written as `{spelled}...`",
            limit + 1,
            &text[..12]
        );
    }
}

/// Indices into what [`held`] returns.
const LABEL: usize = 0;
const CONTENT: usize = 1;
const PARAM: usize = 2;
const DATA: usize = 3;

#[test]
fn a_c_text_of_4095_bytes_is_a_literal_and_one_of_4096_is_its_characters() {
    holds(
        "c11",
        4095,
        &[LABEL, CONTENT, PARAM, DATA],
        &[
            // A call's argument: a compound literal, parenthesized for a macro.
            "((constchar[]){",
            // The send's inline XML: the array of static storage it is copied from.
            "[]={",
            // The host's parameter: the array of static storage its pointer is kept to.
            "[]={",
            // The data's inline XML, handed to the DOM reader.
            "((constchar[]){",
        ],
    );
}

#[test]
fn a_cpp_text_of_65535_bytes_is_a_literal_and_one_of_65536_is_its_characters() {
    holds(
        "cpp",
        65_535,
        &[LABEL, CONTENT, PARAM],
        &["(std::string{", "(std::string{", "(std::string{"],
    );
}
