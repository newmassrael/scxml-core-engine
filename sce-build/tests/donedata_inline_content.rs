// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which of §scxml-B-2's readings inline `<donedata><content>` takes.
//
// The clause orders them: content that is an expression is evaluated,
// and content that is neither XML nor JSON *is a string*. Inline `<data>`
// text has taken both readings since the decision moved to generation
// time; `<donedata>` took only the first, because the parser gave the
// `expr` attribute and the inline body one variant between them. So
// `<content>inline payload</content>` lowered to an expression that could
// not parse, which became a raise inside the artifact — `check --lint`
// reported `expression/unexpected-token` against a document that had
// written no expression, and the machine reached `error.execution`
// instead of carrying its payload.
//
// The two W3C cases are what make the first reading non-negotiable:
// test529 asserts `_event.data == 21` for `<content>21</content>` and
// test294 asserts `_event.data == 'foo'` for `<content>'foo'</content>`.
// A fix that made inline text a string would satisfy the prose in this
// file and fail the conformance suite, so both readings are asserted
// here, and the values are read back off the engine that runs generated
// Rust rather than off the emitted text.

use sce_build::ecmascript::DocumentScope;
use sce_build::filters::{to_author_data_content, to_lua_data_content, to_lua_expr};
use sce_build::model::{DoneDataContent, SCXMLModel};
use sce_build::parser::SCXMLParser;
use sce_rust_lua::LuaEngine;
use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};

/// A one-final document whose `<donedata>` carries `body`, parsed.
fn model_with_content(datamodel: &str, body: &str) -> SCXMLModel {
    let datamodel_attr = if datamodel.is_empty() {
        String::new()
    } else {
        format!(r#" datamodel="{datamodel}""#)
    };
    let source = format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"{datamodel_attr} name="probe" initial="fin">
  <final id="fin"><donedata>{body}</donedata></final>
</scxml>"#
    );
    SCXMLParser::new()
        .parse_string(&source, "probe")
        .unwrap_or_else(|err| panic!("probe document did not parse: {err:?}"))
}

fn content_of(model: &SCXMLModel) -> DoneDataContent {
    model
        .states
        .get("fin")
        .expect("the probe document has a final state")
        .donedata
        .as_ref()
        .expect("the probe document has donedata")
        .content
        .clone()
}

// ── What the parser decides ──────────────────────────────────────

/// The `expr` attribute and the inline body are different constructs and
/// the model says so.
///
/// They were one variant, and the rule belonging to the attribute —
/// evaluate, and raise when you cannot — was applied to the body too.
#[test]
fn the_attribute_and_the_body_are_not_the_same_kind() {
    assert!(
        matches!(
            content_of(&model_with_content("ecmascript", r#"<content expr="'foo'"/>"#)),
            DoneDataContent::Expression(ref text) if text == "'foo'"
        ),
        "<content expr> is the expression form"
    );
    assert!(
        matches!(
            content_of(&model_with_content("ecmascript", "<content>'foo'</content>")),
            DoneDataContent::InlineText(ref text) if text == "'foo'"
        ),
        "inline text under a value expression language is the inline form"
    );
    assert!(
        matches!(
            content_of(&model_with_content("null", "<content>'foo'</content>")),
            DoneDataContent::Literal(ref text) if text == "'foo'"
        ),
        "inline text under the Null data model is the literal form"
    );
}

/// An absent `datamodel` attribute is ECMAScript, so inline text under it
/// takes the ordered readings rather than the verbatim one.
///
/// This is the case the two Null-data-model smoke fixtures were written
/// for and did not reach: their prose named the literal arm while the
/// documents, carrying no attribute, went to the other one.
#[test]
fn an_absent_datamodel_attribute_is_not_the_null_data_model() {
    assert!(
        matches!(
            content_of(&model_with_content("", "<content>plain words</content>")),
            DoneDataContent::InlineText(_)
        ),
        "the platform-specific default is ECMAScript, and inline text under \
         it is not a Null-data-model literal"
    );
}

// ── What the readings evaluate to ────────────────────────────────

/// Both readings, measured on the engine a generated Rust machine runs.
///
/// The pairs are the claim: what an author writes, and what
/// `_event.data` holds because of it.
#[test]
fn inline_text_is_an_expression_when_it_is_one_and_a_string_otherwise() {
    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");
    let session = "donedata_inline".to_string();
    engine.create_session(&session);

    let cases: [(&str, ScriptValue); 6] = [
        // W3C test529 — the number, not the two characters that spell it.
        ("21", ScriptValue::Int(21)),
        // W3C test294 — the string without its quotes.
        ("'foo'", ScriptValue::String("foo".into())),
        // The reading that was unreachable: prose is a string.
        (
            "inline payload",
            ScriptValue::String("inline payload".into()),
        ),
        // A quote inside the prose survives both escapings.
        (
            r#"an inline "payload""#,
            ScriptValue::String(r#"an inline "payload""#.into()),
        ),
        ("true", ScriptValue::Bool(true)),
        // Not an expression, and not a name this datamodel binds either —
        // undeclared names are refused by the frontend, and a refusal is
        // exactly what sends this body to the string reading.
        ("Date", ScriptValue::String("Date".into())),
    ];

    for (body, expected) in cases {
        let model = model_with_content("ecmascript", &format!("<content>{body}</content>"));
        let DoneDataContent::InlineText(text) = content_of(&model) else {
            panic!("<content>{body}</content> did not reach the inline form");
        };
        let scope = DocumentScope::from_model(&model);
        let lua = to_lua_data_content(text, &scope)
            .unwrap_or_else(|err| panic!("<content>{body}</content> did not lower: {err}"));
        match engine.evaluate_expression(&session, &lua) {
            Ok(actual) => assert_eq!(
                actual, expected,
                "<content>{body}</content> lowered to {lua} and evaluated to \
                 something other than the reading §scxml-B-2 gives it"
            ),
            Err(err) => panic!("<content>{body}</content> lowered to {lua}: {err}"),
        }
    }
}

/// The `expr` attribute keeps the reading that belongs to it: an
/// expression it cannot parse raises, because §scxml-5.9.1 says the
/// runtime reports it rather than the build refusing the document.
///
/// This is the half a fix could quietly take away — routing `expr`
/// through the inline filter would turn W3C test 344's deliberate
/// nonsense into a string and silence a conformance case.
#[test]
fn the_expr_attribute_still_raises_on_what_it_cannot_parse() {
    let model = model_with_content("ecmascript", r#"<content expr="return"/>"#);
    let DoneDataContent::Expression(text) = content_of(&model) else {
        panic!("<content expr> did not reach the expression form");
    };
    let scope = DocumentScope::from_model(&model);
    let lua = to_lua_expr(text.clone(), &scope)
        .expect("the expression form lowers to something in every case");
    assert!(
        lua.starts_with("error("),
        "an unparseable <content expr> lowered to {lua}, which does not raise"
    );
    // The same text through the inline filter is a string, which is the
    // whole difference between the two constructs and the reason they
    // cannot share a variant.
    let inline = to_lua_data_content(text, &scope).expect("the inline filter answers every text");
    assert!(
        !inline.starts_with("error("),
        "inline text lowered to {inline}, so the two readings have collapsed again"
    );
}

/// The backends that hand the author's ECMAScript to an ECMAScript
/// engine reach the same reading, in their own language.
///
/// A per-backend answer here is how `<content>21</content>` would come to
/// mean a number in a generated Rust machine and a string in a generated
/// C++ one, from the same document.
#[test]
fn the_verbatim_backends_reach_the_same_reading() {
    let model = model_with_content("ecmascript", "<content>21</content>");
    let scope = DocumentScope::from_model(&model);
    for (body, expected) in [
        ("21", "21"),
        ("'foo'", "'foo'"),
        ("inline payload", "'inline payload'"),
        (r#"an inline "payload""#, r#"'an inline "payload"'"#),
    ] {
        let author = to_author_data_content(body.to_string(), &scope)
            .unwrap_or_else(|err| panic!("<content>{body}</content> did not lower: {err}"));
        assert_eq!(
            author, expected,
            "<content>{body}</content> did not reach the reading its Lua-lowering sibling reaches"
        );
    }
}
