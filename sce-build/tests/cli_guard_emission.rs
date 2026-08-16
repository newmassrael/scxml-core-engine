// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What each backend emits for a `<transition cond>`, and what the model
// promises the emitters about it.
//
// `guard_needs_the_datamodel` pins the decision. This file pins what
// comes out of it, on all six backends and end to end, because the
// decision and the emission were two separate things that disagreed: the
// classifier answered "no data model needed" for any condition it did
// not recognise, and each backend's arm for that answer printed the
// author's text as target-language source. The same document produced
// Rust `if 1 {`, Go `if 1 {`, Kotlin `&& 1 ->`, C++ `if (1)`, C11
// `if (1)` and Python `_scxml_truthy(1)` — three that do not compile,
// two that compile with the host language's truthiness, and one that was
// right. `check` answered exit 0 with no record for all six.
//
// The last claim is the one a text assertion cannot make on its own:
// every guard that reaches this arm has a decided value, so a model that
// stops supplying one turns the emitted literal into a lie rather than a
// compile error. `every_guard_the_backends_emit_natively_has_a_value`
// walks the repository's own documents to say so.

use sce_build::generator::Language;
use sce_build::parser::SCXMLParser;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

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

    fn path(&self) -> &Path {
        &self.0
    }

    /// Every generated artifact's text, concatenated — the emitted
    /// guard lands in a different file per backend and the claim is
    /// about the guard, not the layout.
    fn generated_text(&self) -> String {
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

/// Every backend, derived rather than restated.
///
/// `Language::ALL` says why in its own words: a site that restates the
/// list is how a new backend ends up covered by some all-backends checks
/// and silently skipped by the rest. This file's whole claim is that six
/// emitters agree, so it is exactly the site that must not restate them.
fn backends() -> Vec<&'static str> {
    Language::ALL.iter().map(|l| l.canonical_name()).collect()
}

fn document(cond: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <state id="s0">
    <transition event="go" cond="{cond}" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

struct Generated {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
    text: String,
}

fn generate(cond: &str, language: &str) -> Generated {
    let out = ScratchDir::new(&format!("guard-{language}"));
    let path = out.path().join("guard.scxml");
    std::fs::write(&path, document(cond)).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            language,
            "-o",
            out.path().to_str().unwrap(),
            "--error-format=json",
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    Generated {
        exit: run.status.code(),
        stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
        text: out.generated_text(),
    }
}

/// The probe list is every backend the tool serves.
///
/// A loop cannot see its own list shrink — that is what a shrunken list
/// is for — so the derivation is asserted against the source it derives
/// from, and the spellings against what the CLI actually takes.
#[test]
fn every_backend_the_tool_serves_is_probed() {
    let probed = backends();
    assert_eq!(
        probed.len(),
        Language::ALL.len(),
        "the probe list is not every backend: {probed:?}"
    );
    for name in &probed {
        assert!(
            name.parse::<Language>().is_ok(),
            "{name} is not a spelling `--language` takes"
        );
    }
}

/// A guard SCE decides is emitted as the backend's own boolean, and the
/// author's text survives only as a comment.
///
/// `cond="1"` is ECMA-262-true and is not the token `1` in any of these
/// languages. Asserting the literal is present is not enough on its own —
/// `true` appears all over a generated machine — so the author's text is
/// asserted absent from every line that is not a comment, which is where
/// the defect lived.
#[test]
fn a_decided_guard_is_emitted_as_the_backends_own_boolean() {
    for language in backends() {
        let generated = generate("1", language);
        assert_eq!(
            generated.exit,
            Some(0),
            "{language} stderr:\n{}",
            generated.stderr
        );
        let spliced: Vec<&str> = generated
            .text
            .lines()
            .map(str::trim)
            .filter(|line| !is_comment(line))
            .filter(|line| guards_on(line, "1"))
            .collect();
        assert!(
            spliced.is_empty(),
            "{language} emitted the author's text as source: {spliced:?}"
        );
        // Still a machine with no data model: the whole point of
        // deciding the guard here rather than sending it to an engine.
        let manifest = generated
            .stdout
            .lines()
            .find(|l| l.contains("\"kind\":\"generate\""))
            .expect("a manifest line");
        assert!(
            manifest.contains("\"needs_script_engine\":false"),
            "{language} made a literal guard carry a script engine: {manifest}"
        );
    }
}

/// A guard that names something reaches the frontend, on every backend.
///
/// `cond="x"` used to be emitted as `if x {` with nothing reported. The
/// record asserted here is `expression/unknown-identifier` — a rule that
/// has existed since the resolver landed and that this guard position
/// never reached, because the classifier answered it before the frontend
/// ever saw it.
#[test]
fn a_guard_that_names_something_reaches_the_frontend() {
    for language in backends() {
        let generated = generate("x", language);
        let codes: Vec<String> = generated
            .stderr
            .lines()
            .filter(|l| l.trim_start().starts_with('{'))
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter_map(|r| r["code"].as_str().map(str::to_string))
            .collect();
        assert_eq!(
            codes,
            vec!["expression/unknown-identifier".to_string()],
            "{language} reported {codes:?} for a guard naming nothing, stderr:\n{}",
            generated.stderr
        );
        let spliced: Vec<&str> = generated
            .text
            .lines()
            .map(str::trim)
            .filter(|line| !is_comment(line))
            .filter(|line| guards_on(line, "x"))
            .collect();
        assert!(
            spliced.is_empty(),
            "{language} emitted the author's name as source: {spliced:?}"
        );
    }
}

/// A machine with no data model reaches for no script engine, from any
/// guard position.
///
/// `<if cond="1">` is the position that had no decided arm at all on
/// C11: whatever the classifier answered, the emitter reached for
/// `sm->L` and the header declares that field only for a machine that
/// carries an engine. The generated C did not compile, and nothing said
/// so. Asserted as "no engine is named anywhere" rather than by reading
/// the guard, because the defect was a reference to a thing that is not
/// there.
#[test]
fn a_machine_with_no_data_model_names_no_engine() {
    const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <state id="s0">
    <onentry>
      <if cond="1"><raise event="go"/><elseif cond="0"/><raise event="no"/></if>
    </onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    for language in backends() {
        let out = ScratchDir::new(&format!("no-engine-{language}"));
        let path = out.path().join("no_engine.scxml");
        std::fs::write(&path, DOCUMENT).expect("write document");
        let run = Command::new(sce_codegen_bin())
            .args([
                "generate",
                path.to_str().unwrap(),
                "-l",
                language,
                "-o",
                out.path().to_str().unwrap(),
                "--error-format=json",
            ])
            .current_dir(repo_root())
            .output()
            .expect("spawn sce-codegen");
        assert_eq!(
            run.status.code(),
            Some(0),
            "{language} stderr:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let manifest = String::from_utf8_lossy(&run.stdout)
            .lines()
            .find(|l| l.contains("\"kind\":\"generate\""))
            .map(str::to_string)
            .expect("a manifest line");
        assert!(
            manifest.contains("\"needs_script_engine\":false"),
            "{language} made a decided `<if>` carry a script engine: {manifest}"
        );
        let generated = out.generated_text();
        let reaching: Vec<&str> = generated
            .lines()
            .map(str::trim)
            .filter(|line| !is_comment(line))
            // Call shapes, not names: a machine that carries no engine
            // still *declares* the no-op lifecycle members a machine
            // that does would use, and declaring one reaches for
            // nothing.
            .filter(|line| {
                line.contains("sm->L")
                    || line.contains("luaL_dostring(")
                    || line.contains("ensureScriptEngine();")
                    || line.contains("ensure_script_engine();")
            })
            .collect();
        assert!(
            reaching.is_empty(),
            "{language} reached for an engine this machine does not carry: {reaching:?}"
        );
    }
}

/// Every guard the backends emit without a data model carries a decided
/// value.
///
/// The emitters print `true` or `false` from `cond_constant`, so a guard
/// that reached that arm with `None` would be emitted as `false` — a
/// silent change of meaning rather than a compile error. The classifier
/// and the model field answer one question and this walks the
/// repository's own documents to check they still agree.
#[test]
fn every_guard_the_backends_emit_natively_has_a_value() {
    let root = repo_root();
    let listed = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "resources/*/*.scxml",
            "integration_resources/*/*.scxml",
            "examples/*/*.scxml",
        ])
        .current_dir(&root)
        .output()
        .expect("git ls-files");
    assert!(listed.status.success());
    let documents: Vec<String> = listed
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    // The corpus was 240 documents when this bound was set; a discovery
    // bug that swept nothing would otherwise read as a pass.
    assert!(
        documents.len() >= 200,
        "swept only {} document(s)",
        documents.len()
    );

    let mut guards: Vec<(String, String)> = Vec::new();
    for document in &documents {
        let source = match std::fs::read_to_string(root.join(document)) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let mut parser = SCXMLParser::new();
        let model = match parser.parse_string(&source, document) {
            Ok(model) => model,
            // A document this repository keeps on purpose to be
            // rejected is not a counterexample to what the emitters see.
            Err(_) => continue,
        };
        if model.needs_script_engine {
            continue;
        }
        for state in model.states.values() {
            for transition in &state.transitions {
                if transition.cond.is_empty()
                    || transition.is_pure_in_predicate
                    || transition.is_cpp_condition
                    || transition.is_kt_condition
                {
                    continue;
                }
                guards.push((document.clone(), transition.cond.clone()));
                assert!(
                    transition.cond_constant.is_some(),
                    "{document}: cond=\"{}\" is emitted as a native guard with no \
                     decided value",
                    transition.cond
                );
            }
        }
    }
    // A sweep that reached nothing would pass the loop above without
    // checking anything, so the two documents the corpus is known to
    // carry are named rather than counted: W3C 403a writes
    // `cond="false"` and W3C 449 writes `cond="'foo'"` — the two ends of
    // ECMA-262 9.2 that this arm now decides.
    for (document, cond) in [
        ("resources/403/test403a.scxml", "false"),
        ("resources/449/test449.scxml", "'foo'"),
    ] {
        assert!(
            guards.iter().any(|(d, c)| d == document && c == cond),
            "the sweep did not reach {document}'s cond=\"{cond}\"; it saw {guards:?}"
        );
    }
}

/// Whether a line is only a comment, in any of the six emitted syntaxes.
fn is_comment(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with('#')
        || line.starts_with("/*")
        || line.starts_with('*')
}

/// Whether `line` puts `text` in guard position — the shape the defect
/// took in each language, rather than any mention of the text.
fn guards_on(line: &str, text: &str) -> bool {
    [
        format!("if {text} "),
        format!("if ({text})"),
        format!("if {text}:"),
        format!("&& {text} "),
    ]
    .iter()
    .any(|shape| line.contains(shape.as_str()))
}
