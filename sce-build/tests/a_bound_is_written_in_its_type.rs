// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every bound of a value type is written in the type, at every depth
//! (docs/adr/0086, section 1).
//!
//! # What is held
//!
//! * An event schema's field is read through the one type grammar: every
//!   constructor at every depth up to three, each text, byte string and list
//!   with its bound and without, each outermost bound written in the bracket, as
//!   its legacy attribute, or both, against an oracle of what the grammar and the
//!   position say of it.
//! * The legacy attribute is read as the bracket it is a synonym of, and the
//!   model holds the same field either way; both on one declaration is refused
//!   as `validation/incompatible-attributes`, even when the numbers agree.
//! * A text, byte string or list that states no bound is refused at the place
//!   in the type that lacks it, an event schema's top-level field included.
//! * A composition no backend lowers in a position yet is refused there by
//!   name, `generate/unsupported-feature`, never as a rule of the grammar.
//! * `check` and `generate` report each legacy bound as
//!   `validation/bound-attribute-superseded` and succeed; `--lint` refuses it;
//!   `sce-codegen bound-spelling` prints the document with the bound in its
//!   type, which checks with no record and generates the same code.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::model::{AlgorithmStmt, FieldType, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

fn codegen() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe",
        diagnostic_label: "probe.scxml",
    }
}

fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_value(refusal.error.to_diagnostics()[0].code)
        .expect("a code serialises")
        .as_str()
        .expect("a code is a string")
        .to_string()
}

// ── The oracle over an event schema's field ───────────────────────────────

/// An event schema whose one field `f` is typed `ty` (as the document spells
/// it, `<` escaped) and carries `extra` attributes.
fn schema_with(ty: &str, extra: &str) -> String {
    let ty = ty.replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent">
  <sce:import kind="enum" src="enum_level.scxml" as="Level"/>
  <sce:import kind="event-schema" src="schema_item.scxml" as="Item"/>
  <datamodel>
    <data id="f" sce:type="{ty}" {extra} sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

/// A leaf of the enumeration, and whether it takes a bound.
const LEAVES: &[(&str, bool)] = &[
    ("uint8", false),
    ("int64", false),
    ("float64", false),
    ("bool", false),
    ("enum:Level", false),
    ("record:Item", false),
    ("string", true),
    ("bytes", true),
];

/// One written type, and what the oracle knows of it.
#[derive(Debug, Clone)]
struct Written {
    text: String,
    /// No optional stands directly inside an optional.
    well_formed: bool,
    /// Every bound below the outermost bounded type is stated.
    inner_bounded: bool,
    /// The outermost bounded type, as `(text without its bracket, bracket)`,
    /// or `None` for a type with no length.
    outer: Option<(String, String)>,
    /// A leaf, or an optional over a scalar, a text or a byte string: what an
    /// event schema's field holds in this build.
    field_lowered: bool,
    depth: usize,
}

fn is_record(text: &str) -> bool {
    text.starts_with("record:")
}

/// Every written type up to depth 3: each text, byte string and list once with
/// its bound and once without.
fn written_types() -> Vec<Written> {
    let mut all = Vec::new();
    let mut layer = Vec::new();
    for (leaf, takes_bound) in LEAVES {
        let brackets: &[&str] = if *takes_bound { &["[8]", ""] } else { &[""] };
        for bracket in brackets {
            layer.push(Written {
                text: format!("{leaf}{bracket}"),
                well_formed: true,
                inner_bounded: true,
                outer: takes_bound.then(|| (leaf.to_string(), bracket.to_string())),
                field_lowered: !is_record(leaf),
                depth: 1,
            });
        }
    }
    all.extend(layer.iter().cloned());
    for depth in 2..=3 {
        let mut next = Vec::new();
        for inner in &layer {
            // The outermost bound of `optional<T>` is `T`'s; every other bound
            // of `T` is inside it.
            next.push(Written {
                text: format!("optional<{}>", inner.text),
                well_formed: inner.well_formed && !inner.text.starts_with("optional<"),
                inner_bounded: inner.inner_bounded,
                outer: inner.outer.clone(),
                field_lowered: inner.depth == 1 && !is_record(&inner.text),
                depth,
            });
            for bracket in ["[3]", ""] {
                let inner_stated = inner.inner_bounded
                    && inner
                        .outer
                        .as_ref()
                        .is_none_or(|(_, inner_bracket)| !inner_bracket.is_empty());
                next.push(Written {
                    text: format!("list<{}>{bracket}", inner.text),
                    well_formed: inner.well_formed,
                    inner_bounded: inner_stated,
                    outer: Some((format!("list<{}>", inner.text), bracket.to_string())),
                    field_lowered: false,
                    depth,
                });
            }
        }
        all.extend(next.iter().cloned());
        layer = next;
    }
    all
}

/// How the outermost bound is spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spelling {
    /// In the bracket, as the enumeration wrote it.
    Bracket,
    /// The bracket removed, and the legacy attribute in its place.
    Legacy,
    /// The bracket kept, and the legacy attribute beside it.
    Both,
}

/// What the parser does with a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Held { bound: Option<u32> },
    Refused(&'static str),
}

/// The oracle: what a field typed `w`, spelled `spelling`, comes to.
fn expected(w: &Written, spelling: Spelling) -> Verdict {
    if !w.well_formed {
        return Verdict::Refused(if w.text.starts_with("optional<") {
            "validation/invalid-attribute"
        } else {
            "validation/attribute-rule-violated"
        });
    }
    if spelling == Spelling::Both {
        return Verdict::Refused("validation/incompatible-attributes");
    }
    let outer_stated = match (&w.outer, spelling) {
        (None, _) => true,
        (Some(_), Spelling::Legacy) => true,
        (Some((_, bracket)), _) => !bracket.is_empty(),
    };
    if !(outer_stated && w.inner_bounded) {
        return Verdict::Refused("validation/attribute-rule-violated");
    }
    if !w.field_lowered {
        return Verdict::Refused("generate/unsupported-feature");
    }
    Verdict::Held {
        bound: match (&w.outer, spelling) {
            (None, _) => None,
            (Some(_), Spelling::Legacy) => Some(5),
            (Some(_), _) => Some(8),
        },
    }
}

/// The document of `w` spelled `spelling`, or `None` where the spelling says
/// nothing of the type (no length to bound, or no bracket to keep).
fn spelled(w: &Written, spelling: Spelling) -> Option<String> {
    let attr = |bare: &str| {
        if bare.starts_with("list<") {
            r#"sce:max-count="5""#
        } else {
            r#"sce:max-size="5""#
        }
    };
    match spelling {
        Spelling::Bracket => Some(schema_with(&w.text, "")),
        Spelling::Legacy => {
            let (bare, bracket) = w.outer.as_ref()?;
            if bracket.is_empty() {
                return None;
            }
            let legacy = w.text.replacen(&format!("{bare}{bracket}"), bare, 1);
            Some(schema_with(&legacy, attr(bare)))
        }
        Spelling::Both => {
            let (bare, bracket) = w.outer.as_ref()?;
            if bracket.is_empty() {
                return None;
            }
            Some(schema_with(&w.text, attr(bare)))
        }
    }
}

fn verdict_of(document: &str) -> Verdict {
    match parse_forge(document, label()) {
        Ok(Some(ForgeDocument::EventSchema(schema))) => Verdict::Held {
            bound: schema.fields[0].max_size,
        },
        Ok(other) => panic!("not an event schema: {other:?}"),
        Err(refusal) => Verdict::Refused(match code_of(&refusal).as_str() {
            "validation/invalid-attribute" => "validation/invalid-attribute",
            "validation/attribute-rule-violated" => "validation/attribute-rule-violated",
            "validation/incompatible-attributes" => "validation/incompatible-attributes",
            "generate/unsupported-feature" => "generate/unsupported-feature",
            other => panic!("an unexpected refusal {other}: {refusal}"),
        }),
    }
}

#[test]
fn every_constructor_at_every_depth_is_read_as_the_oracle_says() {
    let cases = written_types();
    // 10 leaves, then three of each type of the depth below.
    assert_eq!(cases.iter().filter(|c| c.depth == 1).count(), 10);
    assert_eq!(cases.iter().filter(|c| c.depth == 2).count(), 30);
    assert_eq!(cases.iter().filter(|c| c.depth == 3).count(), 90);
    let mut seen = std::collections::BTreeMap::<&'static str, usize>::new();
    let mut wrong = Vec::new();
    for case in &cases {
        for spelling in [Spelling::Bracket, Spelling::Legacy, Spelling::Both] {
            let Some(document) = spelled(case, spelling) else {
                continue;
            };
            let want = expected(case, spelling);
            let got = verdict_of(&document);
            *seen
                .entry(match want {
                    Verdict::Held { .. } => "held",
                    Verdict::Refused(code) => code,
                })
                .or_default() += 1;
            if got != want {
                wrong.push(format!(
                    "{} as {spelling:?}: {got:?}, want {want:?}",
                    case.text
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} cases disagree:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // Every verdict is reached, so no branch of the reader is left unasked.
    for verdict in [
        "held",
        "validation/invalid-attribute",
        "validation/attribute-rule-violated",
        "validation/incompatible-attributes",
        "generate/unsupported-feature",
    ] {
        assert!(
            seen.get(verdict).is_some_and(|n| *n > 0),
            "no case reached {verdict}: {seen:?}"
        );
    }
}

#[test]
fn a_field_holds_the_same_type_whichever_spelling_states_its_bound() {
    let field = |document: &str| match parse_forge(document, label()) {
        Ok(Some(ForgeDocument::EventSchema(schema))) => {
            let f = &schema.fields[0];
            (f.sce_type.clone(), f.max_size, f.limit.clone())
        }
        other => panic!("not a schema: {other:?}"),
    };
    for (bracket, legacy, extra, held) in [
        (
            "string[12]",
            "string",
            r#"sce:max-size="12""#,
            FieldType::Scalar(SceType::String),
        ),
        (
            "optional<bytes[4]>",
            "optional<bytes>",
            r#"sce:max-size="4""#,
            FieldType::Optional {
                elem: SceType::Bytes,
            },
        ),
    ] {
        let from_bracket = field(&schema_with(bracket, ""));
        let from_legacy = field(&schema_with(legacy, extra));
        assert_eq!(from_bracket, from_legacy, "{bracket} and {legacy} {extra}");
        assert_eq!(from_bracket.0, held);
    }
    // A named bound: `[name=N]`, or `sce:limit` beside the attribute.
    let named = field(&schema_with("string[probe_text=12]", ""));
    let legacy = field(&schema_with(
        "string",
        r#"sce:max-size="12" sce:limit="probe_text""#,
    ));
    assert_eq!(named, legacy);
    assert_eq!(named.1, Some(12));
    assert_eq!(named.2.as_deref(), Some("probe_text"));
}

#[test]
fn a_top_level_text_with_no_bound_is_refused_at_its_type() {
    for ty in ["string", "bytes", "optional<string>"] {
        let refusal = parse_forge(&schema_with(ty, ""), label()).expect_err(ty);
        assert_eq!(
            code_of(&refusal),
            "validation/attribute-rule-violated",
            "{ty}"
        );
        let said = refusal.to_string();
        assert!(said.contains("has no bound"), "{ty}: {said}");
    }
}

#[test]
fn the_two_spellings_on_one_field_are_refused_even_when_they_agree() {
    for extra in [r#"sce:max-size="8""#, r#"sce:max-size="9""#] {
        let refusal = parse_forge(&schema_with("string[8]", extra), label()).expect_err(extra);
        assert_eq!(
            code_of(&refusal),
            "validation/incompatible-attributes",
            "{extra}"
        );
    }
}

// ── The other positions ───────────────────────────────────────────────────

/// An algorithm returning a `list<int64>` built in a local, the bound of each
/// written as `ret` and `local` say.
fn algorithm(ret: &str, local: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return {ret}/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" {local}/>
    <sce:append target="out" expr="n"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#
    )
}

/// The return's bound and the local's capacity of `document`.
fn algorithm_bounds(document: &str) -> (Option<u32>, Option<u32>) {
    match parse_forge(document, label()) {
        Ok(Some(ForgeDocument::Algorithm(model))) => {
            let local = model.body.iter().find_map(|stmt| match stmt {
                AlgorithmStmt::Var { capacity, .. } => Some(*capacity),
                _ => None,
            });
            (model.signature.returns_max_size, local.flatten())
        }
        other => panic!("not an algorithm: {other:?}"),
    }
}

#[test]
fn an_algorithm_reads_a_buffer_bound_in_either_spelling_and_refuses_both() {
    let legacy = algorithm(
        r#"type="list&lt;int64&gt;" returns-max-size="4""#,
        r#"type="list&lt;int64&gt;" capacity="4""#,
    );
    let bracket = algorithm(
        r#"type="list&lt;int64&gt;[4]""#,
        r#"type="list&lt;int64&gt;[4]""#,
    );
    assert_eq!(algorithm_bounds(&legacy), (Some(4), Some(4)));
    assert_eq!(algorithm_bounds(&bracket), (Some(4), Some(4)));
    for both in [
        algorithm(
            r#"type="list&lt;int64&gt;[4]" returns-max-size="4""#,
            r#"type="list&lt;int64&gt;[4]""#,
        ),
        algorithm(
            r#"type="list&lt;int64&gt;[4]""#,
            r#"type="list&lt;int64&gt;[4]" capacity="4""#,
        ),
    ] {
        let refusal = parse_forge(&both, label()).expect_err("both spellings");
        assert_eq!(code_of(&refusal), "validation/incompatible-attributes");
    }
}

#[test]
fn a_parameter_takes_its_arguments_bound_and_writes_none() {
    let document = algorithm(
        r#"type="list&lt;int64&gt;[4]""#,
        r#"type="list&lt;int64&gt;[4]""#,
    )
    .replace(
        r#"<sce:param name="n" type="int64"/>"#,
        r#"<sce:param name="n" type="int64"/><sce:param name="seen" type="list&lt;int64&gt;[4]"/>"#,
    );
    let refusal = parse_forge(&document, label()).expect_err("a bounded parameter");
    assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
    assert!(
        refusal.to_string().contains("view of its argument"),
        "{refusal}"
    );
}

#[test]
fn a_bound_on_a_slot_of_no_length_is_refused() {
    let returned = algorithm(r#"type="int64[4]""#, r#"type="list&lt;int64&gt;[4]""#);
    let refusal = parse_forge(&returned, label()).expect_err("a bound on a number");
    assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
    let local = algorithm(
        r#"type="list&lt;int64&gt;[4]""#,
        r#"type="list&lt;int64&gt;[4]""#,
    )
    .replace(
        r#"<sce:var name="out" type="list&lt;int64&gt;[4]"/>"#,
        r#"<sce:var name="n2" type="int64" capacity="4" init="0"/><sce:var name="out" type="list&lt;int64&gt;[4]"/>"#,
    );
    let refusal = parse_forge(&local, label()).expect_err("a capacity on a number");
    assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
    assert!(refusal.to_string().contains("only valid on"), "{refusal}");
}

/// An algorithm whose parameter is `param`, whose return is `ret` and which
/// holds one local `local` that starts at `init`, returning `result`.
fn text_algorithm(param: &str, ret: &str, local: &str, init: &str, result: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="given" {param}/>
    <sce:return {ret}/>
  </sce:signature>
  <sce:body>
    <sce:var name="kept" {local} init="{init}"/>
    <sce:return expr="{result}"/>
  </sce:body>
</scxml>
"#
    )
}

/// The bound a held text states, as the model holds it: the local's capacity
/// and the return's, or the refusal's code and sentence.
fn held_bounds(document: &str) -> Result<(Option<u32>, Option<u32>), (String, String)> {
    match parse_forge(document, label()) {
        Ok(Some(ForgeDocument::Algorithm(model))) => {
            let local = model.body.iter().find_map(|stmt| match stmt {
                AlgorithmStmt::Var { capacity, .. } => Some(*capacity),
                _ => None,
            });
            Ok((local.flatten(), model.signature.returns_max_size))
        }
        Ok(other) => panic!("not an algorithm: {other:?}"),
        Err(refusal) => Err((code_of(&refusal), refusal.to_string())),
    }
}

#[test]
fn a_held_text_states_its_bound_and_a_parameter_states_none() {
    // Each text and byte string a return or a local may hold, with no bound and
    // with one: the oracle is that the one with no bound is refused at the type,
    // naming the text, and the other is held at its bound (docs/adr/0086,
    // section 1.4).
    for (bare, bounded, init, place) in [
        ("string", "string[8]", "'a'", "`string` has no bound"),
        (
            "optional&lt;string&gt;",
            "optional&lt;string[8]&gt;",
            "null",
            "`string` has no bound",
        ),
        (
            "optional&lt;bytes&gt;",
            "optional&lt;bytes[8]&gt;",
            "null",
            "`bytes` has no bound",
        ),
    ] {
        let number = r#"type="uint8""#;
        let as_local =
            |ty: &str| text_algorithm(number, number, &format!(r#"type="{ty}""#), init, "0");
        let as_return =
            |ty: &str| text_algorithm(number, &format!(r#"type="{ty}""#), number, "0", init);
        for (slot, unbounded, held) in [
            ("a local", as_local(bare), as_local(bounded)),
            ("a return", as_return(bare), as_return(bounded)),
        ] {
            match held_bounds(&unbounded) {
                Err((code, said)) => {
                    assert_eq!(code, "validation/attribute-rule-violated", "{slot} {bare}");
                    assert!(said.contains(place), "{slot} {bare}: {said}");
                }
                Ok(bounds) => panic!("{slot} {bare} states no bound and was held: {bounds:?}"),
            }
            let bounds = held_bounds(&held).unwrap_or_else(|e| panic!("{slot} {bounded}: {e:?}"));
            let want = if slot == "a local" {
                (Some(8), None)
            } else {
                (None, Some(8))
            };
            assert_eq!(bounds, want, "{slot} {bounded}");
        }
        // A parameter is a view of its argument, of any bound, and states none.
        let viewed = text_algorithm(&format!(r#"type="{bare}""#), number, number, "0", "0");
        assert_eq!(held_bounds(&viewed), Ok((None, None)), "a parameter {bare}");
    }
    // The attributes that stated a text's bound before are its synonyms.
    let legacy = text_algorithm(
        r#"type="uint8""#,
        r#"type="string" returns-max-size="8""#,
        r#"type="string" capacity="8""#,
        "'a'",
        "kept",
    );
    assert_eq!(held_bounds(&legacy), Ok((Some(8), Some(8))));
}

#[test]
fn a_list_of_a_composition_in_an_algorithm_is_refused_by_name() {
    let document = algorithm(
        r#"type="list&lt;string[8]&gt;[4]""#,
        r#"type="list&lt;string[8]&gt;[4]""#,
    );
    let refusal = parse_forge(&document, label()).expect_err("a list of texts");
    assert_eq!(code_of(&refusal), "generate/unsupported-feature");
    assert!(
        refusal.to_string().contains("`string[8]` in an algorithm"),
        "{refusal}"
    );
}

// ── The CLI: reported, linted, and rewritten ──────────────────────────────

fn written(dir: &Path, name: &str, document: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, document).expect("write the document");
    path
}

/// `sce-codegen --error-format=json <args> <path>`, with the records it wrote.
fn run(args: &[&str], path: &Path) -> (Output, Vec<serde_json::Value>) {
    let mut command = codegen();
    command
        .arg("--error-format=json")
        .args(&args[..1])
        .arg(path)
        .args(&args[1..]);
    let output = command.output().expect("spawn sce-codegen");
    let records = String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect();
    (output, records)
}

/// An event schema with no imports whose one field `f` is typed `ty` and
/// carries `extra`: a document `check` and `generate` take alone.
fn plain_schema(ty: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent">
  <datamodel>
    <data id="n" sce:type="uint32" sce:direction="in"/>
    <data id="f" sce:type="{ty}" {extra} sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

/// A `sce-static` statechart holding a list and a text, each bounded by its
/// legacy attribute.
const LEGACY_STATECHART: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="days" sce:type="list&lt;int64&gt;" sce:capacity="4"/>
    <data id="title" sce:type="string" sce:capacity="8" sce:limit="probe_title" expr="'ab'"/>
  </datamodel>
  <state id="s"/>
</scxml>
"#;

/// One attribute the reader takes as a synonym: the document it stands in, the
/// text that starts its declaration there, the declaration as a record names
/// it, the attributes the rewrite removes and the type it writes.
struct Superseded {
    document: String,
    anchor: &'static str,
    element: &'static str,
    attrs: Vec<&'static str>,
    ty: &'static str,
}

/// Each attribute the reader takes as a synonym, once.
fn superseded_cases() -> Vec<Superseded> {
    let legacy_algorithm = algorithm(
        r#"type="list&lt;int64&gt;" returns-max-size="4""#,
        r#"type="list&lt;int64&gt;" capacity="4""#,
    );
    vec![
        Superseded {
            document: plain_schema("string", r#"sce:max-size="12""#),
            anchor: r#"id="f""#,
            element: "<data id=\"f\">",
            attrs: vec!["sce:max-size"],
            ty: "string[12]",
        },
        Superseded {
            document: LEGACY_STATECHART.to_string(),
            anchor: r#"id="days""#,
            element: "<data id=\"days\">",
            attrs: vec!["sce:capacity"],
            ty: "list<int64>[4]",
        },
        Superseded {
            document: LEGACY_STATECHART.to_string(),
            anchor: r#"id="title""#,
            element: "<data id=\"title\">",
            attrs: vec!["sce:capacity", "sce:limit"],
            ty: "string[probe_title=8]",
        },
        Superseded {
            document: legacy_algorithm.clone(),
            anchor: "<sce:return type",
            element: "<sce:return>",
            attrs: vec!["returns-max-size"],
            ty: "list<int64>[4]",
        },
        Superseded {
            document: legacy_algorithm,
            anchor: r#"<sce:var name="out""#,
            element: "<sce:var name=\"out\">",
            attrs: vec!["capacity"],
            ty: "list<int64>[4]",
        },
    ]
}

#[test]
fn check_reports_each_legacy_bound_with_its_rewrite_and_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    for Superseded {
        document,
        anchor,
        element,
        attrs,
        ty,
    } in superseded_cases()
    {
        let path = written(dir.path(), "probe.scxml", &document);
        let (output, records) = run(&["check", "-l", "rust"], &path);
        assert!(
            output.status.success(),
            "{element}: a legacy bound is read, not refused:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "{element}: a run that succeeds prints its manifest"
        );
        let record = records
            .iter()
            .find(|r| r["fix"]["element"] == element)
            .unwrap_or_else(|| panic!("{element}: no record in {records:?}"));
        assert_eq!(record["code"], "validation/bound-attribute-superseded");
        assert_eq!(record["actual"], attrs[0]);
        assert_eq!(record["fix"]["kind"], "move_bound_into_type");
        assert_eq!(record["fix"]["attrs"], serde_json::json!(attrs));
        assert_eq!(record["fix"]["type"], ty);
        // The record stands on the row that holds the attribute.
        let declared = document.find(anchor).expect("the declaration is written");
        let at = declared
            + document[declared..]
                .find(&format!("{}=", attrs[0]))
                .expect("the attribute is written");
        let row = document[..at].matches('\n').count() + 1;
        let on_rows: Vec<u64> = records
            .iter()
            .filter(|r| r["fix"]["element"] == element)
            .filter_map(|r| r["location"]["line"].as_u64())
            .collect();
        assert!(
            on_rows.contains(&(row as u64)),
            "{element}: rows {on_rows:?}, want {row}"
        );
    }
}

#[test]
fn lint_refuses_a_legacy_bound() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written(
        dir.path(),
        "probe.scxml",
        &plain_schema("bytes", r#"sce:max-size="4""#),
    );
    let (output, records) = run(&["check", "-l", "rust", "--lint"], &path);
    assert!(!output.status.success(), "--lint makes the record fatal");
    assert!(output.stdout.is_empty(), "a refused run prints no manifest");
    let codes: Vec<&serde_json::Value> = records.iter().map(|r| &r["code"]).collect();
    assert_eq!(
        codes,
        [&serde_json::json!("validation/bound-attribute-superseded")]
    );
}

#[test]
fn bound_spelling_prints_a_document_that_checks_with_no_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    for Superseded {
        document,
        element,
        ty,
        ..
    } in superseded_cases()
    {
        let path = written(dir.path(), "probe.scxml", &document);
        let printed = codegen()
            .arg("bound-spelling")
            .arg(&path)
            .output()
            .expect("spawn sce-codegen");
        assert!(
            printed.status.success(),
            "{element}: bound-spelling refused:\n{}",
            String::from_utf8_lossy(&printed.stderr)
        );
        let rewritten = String::from_utf8(printed.stdout).expect("UTF-8");
        let escaped = ty.replace('<', "&lt;").replace('>', "&gt;");
        assert!(
            rewritten.contains(&format!("=\"{escaped}\"")),
            "{element}: the type carries its bound:\n{rewritten}"
        );
        let path = written(dir.path(), "probe.scxml", &rewritten);
        let (output, records) = run(&["check", "-l", "rust"], &path);
        assert!(
            output.status.success() && records.is_empty(),
            "{element}: the rewritten document carries no legacy bound:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// `text` with every run of 64 lowercase hexadecimal characters, a source hash,
/// replaced: two documents of one meaning hash apart.
fn without_hashes(text: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    for c in text.chars() {
        if c.is_ascii_hexdigit() && !c.is_ascii_uppercase() {
            run.push(c);
            continue;
        }
        out.push_str(if run.len() == 64 { "<hash>" } else { &run });
        run.clear();
        out.push(c);
    }
    out.push_str(if run.len() == 64 { "<hash>" } else { &run });
    out
}

/// What `sce-codegen generate -l rust` writes for `document`, file by file.
fn generated(document: &str) -> Vec<(String, String)> {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = tempfile::tempdir().expect("tempdir");
    let path = written(dir.path(), "probe.scxml", document);
    let run = codegen()
        .arg("generate")
        .arg(&path)
        .args(["-l", "rust", "-o"])
        .arg(out.path())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "generate refused:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    // Where each run's input and output stand is no part of what it generated.
    let places = [
        dir.path().to_string_lossy().into_owned(),
        out.path().to_string_lossy().into_owned(),
    ];
    let mut files: Vec<(String, String)> = std::fs::read_dir(out.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.is_file())
        .map(|path| {
            let mut text = std::fs::read_to_string(&path).unwrap_or_default();
            for place in &places {
                text = text.replace(place.as_str(), "<dir>");
            }
            (
                path.file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned(),
                without_hashes(&text),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn a_rewritten_document_generates_what_the_legacy_one_did() {
    let legacy = plain_schema("string", r#"sce:max-size="12""#);
    let bracket = plain_schema("string[12]", "");
    let from_legacy = generated(&legacy);
    assert!(!from_legacy.is_empty(), "the schema generates a unit");
    assert_eq!(from_legacy, generated(&bracket));
}
