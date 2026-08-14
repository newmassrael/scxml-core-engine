// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.3: a lowered `<data>` variable is readable, and it is stored once.
//
// SCE used to lower every scalar `<data>` into a typed field on the generated
// machine — `max_turns: i64`, initialised to the document's literal — and emit
// no way to read it. A consumer could not ask a machine what its own budget
// was; the only readable copy was the script engine's, reached with an engine
// handle, a session id and the variable's name spelled as a string. That is
// the gap sprag reported and worked around in
// `crates/sprag-plugin/src/ai_loop.rs`.
//
// The field was worse than useless, and this file's first test is why. A
// `<data>` is typed only when it carries an initialiser
// (`analyzer::classify_variables`), and an initialiser is itself what makes a
// document need a script engine
// (`script_engine_analyzer`, `DatamodelVariableInit`). So the typed field
// existed exactly when the script engine already owned the variable — one
// variable with two representations, of which only the engine's could ever be
// right, since `<assign>` writes there. Kotlin's template had already gated
// the field on `not needs_script_engine`, which by that implication closed it
// entirely; the other four backends emitted it unconditionally.
//
// The invariant is asserted rather than assumed because the accessor's shape
// depends on it: an accessor reads through the script engine, so a typed
// variable that arrived WITHOUT one would emit code referencing a field that
// does not exist. That would be a compile failure in generated code — loud,
// but late and far from its cause. Here it fails at its cause.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sce_build::parser::SCXMLParser;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Parse and analyse a document the way the compile entry points do.
fn model_of(content: &str, label: &str) -> sce_build::model::SCXMLModel {
    let mut parser = SCXMLParser::new();
    let mut model = parser
        .parse_string(content, label)
        .unwrap_or_else(|e| panic!("parse failed for {label}: {:?}", e.error));
    sce_build::analyzer::analyze(&mut model, "");
    model
}

fn doc_with_data(data: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="probe" datamodel="ecmascript" initial="a">
  <datamodel>{data}</datamodel>
  <state id="a"/>
</scxml>"#
    )
}

/// The typed set, spelled once. `runtime` is the classifier's fourth answer
/// and the one that carries no host type, so it is the complement rather than
/// a fifth name.
///
/// Scope, measured rather than assumed: `classify_variables` walks
/// `model.variables`, which is the document-level `<datamodel>` only. A
/// state-scoped `<data>` never carried a type, so it never had a shadow field
/// and gets no accessor — the emission follows the typed set exactly rather
/// than drawing a second, narrower line of its own.
const TYPED: [&str; 3] = ["int", "string", "bool"];

fn typed_var_ids(model: &sce_build::model::SCXMLModel) -> Vec<String> {
    model
        .variables
        .iter()
        .filter(|v| TYPED.contains(&v.var_type.as_str()))
        .map(|v| v.id.clone())
        .collect()
}

/// A typed datamodel variable always arrives with a script engine.
///
/// Probed across every branch of the classifier rather than by example: the
/// implication has to hold for the branch that produces each host type, and a
/// single fixture would only exercise whichever branch it happened to use.
#[test]
fn a_typed_datamodel_variable_always_comes_with_a_script_engine() {
    let branches = [
        ("int", r#"<data id="v" expr="40"/>"#),
        ("int (negative)", r#"<data id="v" expr="-7"/>"#),
        ("int (zero)", r#"<data id="v" expr="0"/>"#),
        ("string", r#"<data id="v" expr="&quot;hello&quot;"/>"#),
        ("bool (true)", r#"<data id="v" expr="true"/>"#),
        ("bool (false)", r#"<data id="v" expr="false"/>"#),
    ];

    for (label, data) in branches {
        let model = model_of(&doc_with_data(data), label);
        let typed = typed_var_ids(&model);
        assert_eq!(
            typed,
            vec!["v".to_string()],
            "the {label} branch must produce a typed variable, or this probe is \
             asserting nothing about it"
        );
        assert!(
            model.needs_script_engine,
            "⚠ the {label} branch produced a typed datamodel variable WITHOUT a \
             script engine. Every typed accessor reads through the engine, so a \
             machine in that shape would emit an accessor over a handle it does \
             not have. Repair the accessor emission before this branch can land."
        );
    }
}

/// The type of a `<data>` initialiser is read off the value it denotes, not
/// off the characters it is spelled with.
///
/// The classifier this replaced tested the first and last character for `"`,
/// which is a question about quoting rather than about type. ECMAScript has
/// two spellings for one string literal, and the AI-loop example uses the
/// other one for every string it declares — so its whole editable strategy
/// block, the part of the document a host is meant to edit and read back,
/// reached consumers with no accessor at all. Nothing failed; the accessors
/// were simply absent, which is the shape of gap this file exists to refuse.
///
/// The pairs below are the cases where a character test and a parse disagree.
/// Each is asserted with its counterpart so a repair that merely added `'` to
/// the character test would still fail here: `'42'` and `42` are spelled
/// almost alike and are not the same type, and `1e3` and `1.5` are spelled
/// almost alike and are not either.
#[test]
fn an_initialiser_is_typed_by_what_it_denotes_not_by_how_it_is_spelled() {
    // (label, the `expr` attribute's text, the type it declares)
    let cases: [(&str, &str, &str); 16] = [
        ("double-quoted string", r#"&quot;hello&quot;"#, "string"),
        ("single-quoted string", "'hello'", "string"),
        ("empty string", "''", "string"),
        // A string of digits is a string. A character test that accepted
        // `'…'` by its quotes gets this one right; one that stripped quotes
        // and re-sniffed would not.
        ("digits inside quotes", "'42'", "string"),
        // An escape makes the literal's interior look like anything at all,
        // including a quote. The parse decodes it; a scan over the raw text
        // is reading the escape as content.
        ("string with an escaped quote", r#"'it\'s'"#, "string"),
        ("string with a newline escape", r#"'a\nb'"#, "string"),
        ("integer", "40", "int"),
        ("zero", "0", "int"),
        ("negation", "-7", "int"),
        ("unary plus", "+7", "int"),
        ("hexadecimal", "0x1f", "int"),
        ("exponent denoting a whole number", "1e3", "int"),
        ("boolean", "false", "bool"),
        // Not integers: ECMAScript has one number type, but the accessor set
        // has no reader for a fractional value, so these declare no type
        // rather than an accessor that would answer `None` for ever.
        ("fraction", "1.5", "runtime"),
        ("exponent beyond i64", "1e400", "runtime"),
        // Not a literal at all.
        ("array", "[1,2,3]", "runtime"),
    ];

    for (label, expr, expected) in cases {
        let model = model_of(
            &doc_with_data(&format!(r#"<data id="v" expr="{expr}"/>"#)),
            label,
        );
        let actual = &model.variables[0].var_type;
        assert_eq!(
            actual,
            expected,
            "⚠ `<data expr=\"{expr}\"/>` ({label}) was classified `{actual}`, so the \
             emitted accessor set is {}. The classifier must answer from the \
             parsed initialiser, not from its punctuation.",
            if expected == "runtime" {
                "wider than the types it can honour"
            } else {
                "missing a reader a consumer needs"
            }
        );
    }
}

/// An expression whose result type ECMA-262 fixes is typed; one that would
/// need its value computed is not.
///
/// `+` is the operator the AI-loop example builds its prompts with, and it is
/// also the only one whose result type a single operand decides: ECMA-262 3rd
/// ed. §11.6.1 concatenates whenever either side is a String, whatever the
/// other side holds. That is why `'Milestone: ' + milestone` can be typed
/// without knowing what `milestone` is, while `1 + 2` is deliberately left
/// alone — folding arithmetic here would make the classifier a second
/// evaluator, and the two would disagree at the edges the first one owns.
#[test]
fn a_computed_initialiser_is_typed_only_where_the_language_settles_it() {
    let cases: [(&str, &str, &str); 9] = [
        ("string on the left", "'a: ' + other", "string"),
        ("string on the right", "other + ' :b'", "string"),
        (
            "string reached through a chain",
            "'a' + b + c + d",
            "string",
        ),
        // Both arms agree, so which one wins never has to be decided.
        ("conditional agreeing on string", "c ? 'a' : 'b'", "string"),
        ("either-or agreeing on string", "'' || 'fallback'", "string"),
        ("conditional disagreeing", "c ? 'a' : 7", "runtime"),
        // The defaulting idiom as it is actually written: `a` decides the
        // result whenever it is truthy, and nothing here says what `a` holds.
        // Typing this `string` from the visible operand would be a promise
        // about the invisible one.
        (
            "either-or over an unknown operand",
            "a || 'fallback'",
            "runtime",
        ),
        // Not settled without computing a value.
        ("integer addition", "1 + 2", "runtime"),
        ("subtraction of strings", "'a' - 'b'", "runtime"),
    ];

    for (label, expr, expected) in cases {
        let model = model_of(
            &doc_with_data(&format!(r#"<data id="v" expr="{expr}"/>"#)),
            label,
        );
        let actual = &model.variables[0].var_type;
        assert_eq!(
            actual, expected,
            "⚠ `<data expr=\"{expr}\"/>` ({label}) was classified `{actual}`, not \
             `{expected}`. A type claimed here is a promise the runtime accessor \
             has to keep, and a type withheld here is an accessor a consumer does \
             not get."
        );
    }
}

/// The example the AI-loop axis ships is readable in the half that is meant
/// to be edited.
///
/// This asserts over the committed document rather than over a fixture,
/// because the gap was never visible in a fixture: every hand-written probe
/// in this file used the double-quoted spelling, so the classifier's blind
/// spot sat under a green suite while the document the axis exists to
/// demonstrate had eight unreadable variables — every string it declares.
///
/// `screen_rules` is the one declaration named as an exclusion here rather
/// than left to be noticed: it is an array, and the typed accessor set has no
/// reader for one. If a later change gives it one, this list is where that
/// shows up.
#[test]
fn the_ai_loop_examples_editable_strategy_is_readable() {
    let doc = repo_root().join("examples/ai_loop/ai_loop.scxml");
    let content = std::fs::read_to_string(&doc).expect("the AI-loop example is committed");
    let model = model_of(&content, "examples/ai_loop/ai_loop.scxml");

    let typed: BTreeSet<String> = typed_var_ids(&model).into_iter().collect();
    let declared: BTreeSet<String> = model.variables.iter().map(|v| v.id.clone()).collect();
    let untyped: Vec<&String> = declared.difference(&typed).collect();

    assert_eq!(
        untyped,
        vec![&"screen_rules".to_string()],
        "⚠ the example's datamodel declares {} variables and {} of them carry a \
         typed accessor. Everything but `screen_rules` — an array, which no \
         accessor reads — must be answerable, because the strategy strings are \
         exactly what a supervising host edits and then needs to read back.",
        declared.len(),
        typed.len()
    );

    // The floor keeps the assertion above from passing over a document that
    // shrank: `untyped` is also a one-element vector when the datamodel holds
    // nothing but `screen_rules`.
    assert!(
        declared.len() >= 16,
        "the example declares only {} datamodel variables, so this test is no \
         longer measuring the block it names",
        declared.len()
    );
}

/// The complement, so the implication above is not vacuously easy to keep: a
/// `<data>` with no initialiser is the one shape that stays untyped, and it is
/// also the only one that can appear without a script engine.
#[test]
fn a_data_without_an_initialiser_is_untyped_and_needs_no_engine() {
    let model = model_of(&doc_with_data(r#"<data id="v"/>"#), "bare");

    assert!(
        typed_var_ids(&model).is_empty(),
        "a `<data>` with nothing to evaluate carries no host type, so there is \
         nothing for a typed accessor to return: {:?}",
        model
            .variables
            .iter()
            .map(|v| &v.var_type)
            .collect::<Vec<_>>()
    );
    assert!(
        !model.needs_script_engine,
        "and it is the shape that reaches codegen without an engine at all — \
         which is what makes the implication above worth asserting"
    );
}

/// The implication again, this time over every document the repository
/// commits, so a corpus shape no hand-written probe thought of is covered too.
///
/// The scan set comes from `git ls-files` rather than from a list of
/// directories, and that is a correction rather than a preference: the first
/// version of this test named four trees, one of which (`tests/w3c/resources`)
/// does not exist. It reached 75 of the corpus's documents and would have
/// reported "no violations" over the fifth of it that it saw.
///
/// The lower bounds are the guard against exactly that. A tree that moved, a
/// filter that stopped matching, or a parser that started refusing the corpus
/// would all leave this green over a set too small to mean anything.
#[test]
fn no_committed_document_declares_a_typed_variable_without_an_engine() {
    let mut typed_seen = 0usize;
    let mut docs_parsed = 0usize;
    let mut violations: BTreeSet<String> = BTreeSet::new();

    for entry in committed_scxml() {
        let Ok(content) = std::fs::read_to_string(&entry) else {
            continue;
        };
        let label = entry.display().to_string();
        let mut parser = SCXMLParser::new();
        // A document this sweep cannot parse is not this test's finding — the
        // parser suites own that — so it is skipped rather than counted. The
        // lower bound below is what keeps the skip honest.
        let Ok(mut model) = parser.parse_string(&content, &label) else {
            continue;
        };
        sce_build::analyzer::analyze(&mut model, "");
        docs_parsed += 1;
        let typed = typed_var_ids(&model);
        typed_seen += typed.len();
        if !typed.is_empty() && !model.needs_script_engine {
            violations.insert(format!("{label}: {typed:?}"));
        }
    }

    // What the sweep actually reached, printed rather than only asserted: a
    // reader setting the floors below needs the current numbers, and a run
    // whose scan set shrank without crossing a floor is visible here first.
    eprintln!("datamodel sweep: {docs_parsed} document(s), {typed_seen} typed variable(s)");

    // Measured 2026-08-14: 705 committed `.scxml`, of which this sweep parses
    // 443 and finds 96 typed datamodel variables. The gap is the documents
    // `parse_string` declines without filesystem context — preprocessor
    // templates, `<data src>`, forge kinds — and it is stated rather than
    // hidden: this test speaks for the 443 it reads, and the floors below sit
    // under those numbers so a fixture can be retired without a false alarm
    // while a silently emptied sweep still cannot report success.
    assert!(
        docs_parsed >= 400,
        "the sweep parsed only {docs_parsed} committed documents, so it is not \
         measuring the corpus it claims to — check the scan set and the parser \
         before reading its verdict"
    );
    assert!(
        typed_seen >= 80,
        "the sweep found only {typed_seen} typed datamodel variables across \
         {docs_parsed} documents. A classifier change that stopped typing \
         anything would make this test pass while asserting nothing"
    );
    assert!(
        violations.is_empty(),
        "these committed documents declare a typed datamodel variable with no \
         script engine, which the accessor emission cannot express: {violations:#?}"
    );
}

/// Every `.scxml` the repository tracks, asked of git rather than of the
/// filesystem so build output and scratch trees cannot join the scan set.
fn committed_scxml() -> Vec<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(["ls-files", "*.scxml"])
        .output()
        .expect("git ls-files");
    assert!(
        out.status.success(),
        "git ls-files failed, so this sweep has no scan set to measure"
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|rel| repo_root().join(rel))
        .collect()
}

/// Every backend emits the accessor, and none of them keeps the shadow.
///
/// One document, six languages, two claims each. The pairing is the point: an
/// accessor without the removal would leave the two representations in place
/// and only add a third way to read one of them, and the removal without the
/// accessor is the gap this whole file exists to close.
///
/// C11 is asserted for the removal half only, and says so below rather than
/// being quietly dropped from the list.
#[test]
fn every_backend_reads_the_datamodel_and_none_shadows_it() {
    use sce_build::generator::Language;

    let dir = tempfile::tempdir().expect("tempdir");
    let doc = dir.path().join("probe.scxml");
    std::fs::write(
        &doc,
        doc_with_data(
            r#"
    <data id="max_turns" expr="40"/>
    <data id="label" expr="&quot;idle&quot;"/>
    <data id="screening" expr="false"/>"#,
        ),
    )
    .expect("write probe");

    // (language, the accessor's call into the shared read helper, the shadow
    // field's declaration as that backend used to spell it)
    let expectations: [(Language, &str, Option<&str>); 6] = [
        (
            Language::Rust,
            "helpers::datamodel_read::read_int",
            Some("max_turns: i64"),
        ),
        (
            Language::Cpp,
            "DataModelReadHelper::readInt",
            Some("int max_turns"),
        ),
        (Language::Go, "sce.ReadDatamodelInt", Some("maxTurns int64")),
        (
            Language::Kotlin,
            "DatamodelRead.readInt",
            Some("var maxTurns"),
        ),
        (
            Language::Python,
            "_datamodel_read.read_int",
            Some("self.max_turns"),
        ),
        // C11 never lowered a `<data>` into a struct member at all, so it has
        // no shadow to remove and no typed member an accessor could read
        // back. Named here so the omission is a recorded measurement rather
        // than a backend someone forgot.
        (Language::C11, "", None),
    ];

    for (language, accessor_call, shadow) in expectations {
        let out = sce_build::compile_scxml_lang(
            doc.to_str().expect("utf-8 path"),
            &sce_build::find_template_dir_for(language),
            language,
        )
        .unwrap_or_else(|e| panic!("{language:?} codegen failed: {e}"));
        let emitted: String = out.files.iter().map(|(_, body)| body.as_str()).collect();

        if !accessor_call.is_empty() {
            assert!(
                emitted.contains(accessor_call),
                "⚠ {language:?} emitted no read accessor for a typed `<data>`. A \
                 consumer of this backend still cannot ask a machine what its own \
                 datamodel holds, which is the gap this emission closes."
            );
        }
        if let Some(shadow) = shadow {
            assert!(
                !emitted.contains(shadow),
                "⚠ {language:?} still declares `{shadow}` alongside the session's \
                 copy. One variable with two representations goes wrong at the \
                 first `<assign>`, and the accessor above would then have a \
                 wrong-but-plausible neighbour for a consumer to reach for."
            );
        }
    }
}
