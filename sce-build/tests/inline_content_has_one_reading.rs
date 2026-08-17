// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Inline `<content>` text is read once, at build time, wherever the
// clause puts it.
//
// W3C SCXML §5.6 says the Processor uses the *children* of `<content>`
// as the output and that "the interpretation of the output depends on
// the data model" — on the data model, not on the parent element. So
// `<send>`, `<donedata>` and `<data>` owe the same text the same
// reading, and Appendix B.2 is what gives it: an expression when it is
// one, XML when it opens with `<`, otherwise the string.
//
// `<donedata>` and `<data>` reached that reading. `<send>` did not, and
// what stood in its place was five different searches performed at run
// time, one per backend:
//
//   rust, go   the author's text evaluated *as Lua* — `t.length` is nil
//              where the datamodel reads 5
//   python     the author's text returned unread, so `<content>123`
//              was the string "123"
//   cpp        `jsonStringToScriptValue`, which accepts JSON and
//              nothing else
//   kotlin     a four-step cascade: XML, then a JS eval, then
//              `JSON.parse`, then the trimmed string
//   c11        generated Lua that rewrote `"k":` into `["k"]=`, ran
//              `load('return (' .. text .. ')')`, and fell back to the
//              trimmed string
//
// Five answers to one document, and no diagnostic on any of them. What
// let it stay that way is that nothing compared the backends: W3C 179
// writes `<content>123</content>` and asserts `_event.data == 123` with
// the *loose* equality, which the string "123" also satisfies.
//
// This file is that comparison. It reads what each backend emits for one
// document and requires the six to carry the same reading — in whichever
// language that backend evaluates.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use sce_build::generator::Language;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }

    fn text(&self) -> String {
        let mut all = String::new();
        for entry in std::fs::read_dir(&self.0).expect("read scratch dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == "json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                all.push_str(&text);
            }
        }
        all
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A document whose only payload is one inline `<content>` body.
fn document(content: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <datamodel><data id="t" expr="'hello'"/></datamodel>
  <state id="s0">
    <onentry><send event="e1"><content>{content}</content></send></onentry>
    <transition event="e1" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// Everything `language` emits for a document carrying `content`.
fn emitted(content: &str, language: &str) -> String {
    let out = ScratchDir::new(&format!("inline-content-{language}"));
    let path = out.0.join("inline_content.scxml");
    std::fs::write(&path, document(content)).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            language,
            "-o",
            out.0.to_str().unwrap(),
            "--error-format=json",
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "{language} refused the document:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    out.text()
}

/// `text` with comments removed, in the three syntaxes the six backends
/// emit: `//`, `#`, and `/* … */`.
///
/// The block form needs a state machine rather than a line filter, and
/// finding that out cost two runs of this file: the C11 emitter explains
/// what it replaced in a multi-line `/* … */`, whose *continuation*
/// lines start with ordinary prose. A line filter passed them straight
/// through and this file reported the very search whose removal it was
/// checking.
fn without_comments(text: &str) -> String {
    // `#` is deliberately only a *whole-line* comment: Python spells its
    // comments that way, and Lua spells the length operator `#t` — which
    // is one of the readings this file looks for. Treating every `#` as a
    // comment would erase the evidence.
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let mut at_line_start = true;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        if (c == '/' && next == Some('/')) || (c == '#' && at_line_start) {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if !c.is_whitespace() {
            at_line_start = false;
        } else if c == '\n' {
            at_line_start = true;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// The two languages a backend evaluates the reading in.
///
/// Four backends lower the author's ECMAScript to Lua and evaluate that;
/// C++ and Kotlin hand the author's own text to an ECMAScript engine.
/// The reading is the same either way — this is what each *spells* it
/// as, and a backend that carries neither spelling is carrying the
/// author's text unread.
fn expected_spellings(language: &str) -> [&'static str; 1] {
    match language {
        "cpp" | "kotlin" => ["t.length"],
        _ => ["#t"],
    }
}

/// The comparison spans every backend the tool serves.
///
/// A loop cannot see its own list shrink, so the list is derived from
/// `Language::ALL` and the derivation is what gets asserted. Written
/// after a mutation restricted one of the loops below to a single
/// backend and nothing went red: the other five were simply not
/// compared, which is the state this whole file exists to end.
#[test]
fn every_backend_the_tool_serves_is_compared() {
    let probed: Vec<&str> = Language::ALL.iter().map(|l| l.canonical_name()).collect();
    assert_eq!(
        probed.len(),
        Language::ALL.len(),
        "the comparison does not span every backend: {probed:?}"
    );
    for name in &probed {
        assert!(
            name.parse::<Language>().is_ok(),
            "{name} is not a spelling `--language` takes"
        );
        // Every backend evaluates the reading in one of the two
        // languages, and a backend in neither column would be compared
        // against a spelling nothing emits.
        let [spelling] = expected_spellings(name);
        assert!(
            spelling == "#t" || spelling == "t.length",
            "{name} is compared against {spelling}, which is neither reading"
        );
    }
}

/// Every backend carries the same reading of an expression.
///
/// `t.length` is the case that separates a reading from a copy: it is
/// valid ECMAScript, valid *Lua syntax*, and means different things in
/// the two — 5 against nil. A backend that passes the author's text
/// through unread therefore compiles and runs and answers wrong, which
/// is why the assertion is about the lowered spelling rather than about
/// the text being present.
#[test]
fn every_backend_reads_an_expression_the_same_way() {
    for language in Language::ALL.iter().map(|l| l.canonical_name()) {
        let text = emitted("t.length", language);
        let [spelling] = expected_spellings(language);
        assert!(
            text.contains(spelling),
            "{language} does not carry the reading `{spelling}` for \
             <content>t.length</content> — it is passing the author's text \
             on unread"
        );
    }
}

/// Text that is not an expression is the string, on every backend.
///
/// The other half of B.2's ordering, and the half a reading can get
/// wrong in the opposite direction: a rule that evaluated everything
/// would turn prose into an error or a nil. `Date` is the sharp case —
/// it is a name ECMA-262 defines and SCE does not install, so reading it
/// as an expression binds nothing while the clause says it is the text.
#[test]
fn every_backend_reads_prose_as_the_string() {
    for language in Language::ALL.iter().map(|l| l.canonical_name()) {
        for (content, quoted) in [("inline payload", "inline payload"), ("Date", "Date")] {
            let text = emitted(content, language);
            let quoted_forms = [
                format!("\"{quoted}\""),
                format!("'{quoted}'"),
                format!("\\\"{quoted}\\\""),
            ];
            assert!(
                quoted_forms.iter().any(|form| text.contains(form.as_str())),
                "{language} does not carry <content>{content}</content> as a \
                 string literal — B.2 makes text that is not an expression the \
                 string"
            );
        }
    }
}

/// The send path no longer searches for the reading at run time.
///
/// Named per backend rather than swept for, and the difference matters:
/// a generic search for `load('return (` also finds the *receive*-side
/// decoder, which turns a transported payload back into a value and is a
/// different seam with a different job — data arriving from another
/// session or an HTTP processor was never read from this document. The
/// first version of this test conflated the two and reported C11's
/// event-delivery decoder as a send-side defect.
///
/// So each term below is the tell of the search that stood in *this*
/// backend's send path, and a backend with no named term is one whose
/// defect was passing the text through unread — which the two tests
/// above are what catch.
fn send_side_search(language: &str) -> &'static [&'static str] {
    match language {
        // The JSON-only sniff that decided the value beside the string.
        "cpp" => &["jsonStringToScriptValue"],
        // The generated chunk that rewrote `"k":` into `["k"]=` and then
        // tried `load`, assigning whichever won to `_pending_donedata`.
        "c11" => &["_pending_donedata = _val"],
        _ => &[],
    }
}

/// What a backend's send path must *contain* for the reading to be
/// evaluated rather than passed on.
///
/// Python needs the positive form. Its defect was not a search but the
/// refusal to make one — the helper returned the content it was handed —
/// and that is invisible in the emitted *call*, which carries the reading
/// either way. Asserting the absence of `return content` looked equivalent
/// and was not: the XML reading returns its source on purpose, so the
/// absent-form condemned the arm B.2 requires. The evaluation is the thing
/// to require, so it is what gets required.
fn send_side_evidence(language: &str) -> &'static [&'static str] {
    match language {
        "python" => &["self._session_id, content"],
        _ => &[],
    }
}

#[test]
fn the_send_path_no_longer_searches_for_the_reading() {
    for language in Language::ALL.iter().map(|l| l.canonical_name()) {
        // Comments first: the emitted sources explain what they replaced,
        // and a scan that read those would report the very searches their
        // removal is the subject of. This assertion caught its own
        // explanatory comment, twice — once as a line comment and once as
        // the prose continuation of a block comment.
        let text = without_comments(&emitted("t.length", language));
        for search in send_side_search(language) {
            assert!(
                !text.contains(search),
                "{language} still searches for the reading at run time: {search}"
            );
        }
        for evidence in send_side_evidence(language) {
            assert!(
                text.contains(evidence),
                "{language} does not evaluate the reading it was handed: \
                 `{evidence}` is absent"
            );
        }
    }
}
