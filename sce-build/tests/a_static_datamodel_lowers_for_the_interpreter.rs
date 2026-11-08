// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"` lowered to the ecmascript document the Interpreter
// runs (docs/SCE_ACCEPTED_SUBSET.md §2.15) — every statechart the shared
// fixture directory holds, through the same entry `sce-codegen lower` uses.
//
// What is held here is the lowering's own contract, with no engine to run it:
// a document is either lowered to something the Interpreter can read, or
// refused with `generate/unsupported-feature` naming the construct — never
// passed through half lowered. Whether the lowered document then BEHAVES as
// the generated backends do is the C++ suite's question
// (`tests/integration/AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`,
// which replays the scenarios the AOT backends replay).
//
// Nothing is listed: the fixtures are found by what they declare, so a new
// one is judged the day it is added.

use std::path::{Path, PathBuf};

use sce_build::forge::static_js::{
    lower_documents, lower_file, lower_file_set, lower_source, LoweredDocument,
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// Every statechart under the fixture directory that declares
/// `datamodel="sce-static"`, sorted so a failure names the same one each run.
fn static_statecharts() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("the fixture directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .filter(|path| {
            std::fs::read_to_string(path)
                .expect("a readable fixture")
                .contains(r#"datamodel="sce-static""#)
        })
        .collect();
    found.sort();
    found
}

fn scxml_element<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children().find(|n| {
        n.is_element()
            && n.tag_name().name() == name
            && n.tag_name().namespace() == Some("http://www.w3.org/2005/07/scxml")
    })
}

#[test]
fn every_fixture_is_lowered_or_refused_by_name() {
    let mut lowered = Vec::new();
    let mut refused = Vec::new();
    for path in static_statecharts() {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        // The set, not the document: a hybrid `<invoke>` starts a document of
        // its own, which is lowered beside the one that invokes it.
        match lower_file_set(path.to_str().unwrap(), Vec::new()) {
            Ok(documents) => {
                for LoweredDocument { file_name, text } in &documents {
                    let name = format!("{name} ({file_name})");
                    let document = text;
                    let parsed = roxmltree::Document::parse(document)
                        .unwrap_or_else(|e| panic!("{name}: the lowered document is not XML: {e}"));
                    let root = parsed.root_element();
                    assert_eq!(
                        root.attribute("datamodel"),
                        Some("ecmascript"),
                        "{name}: the Interpreter runs an ecmascript document"
                    );
                    // The library is installed before anything that reads it,
                    // and only when something does.
                    let calls_the_library = document.contains("SceStatic.");
                    let first_data = scxml_element(root, "datamodel")
                        .and_then(|datamodel| scxml_element(datamodel, "data"));
                    if calls_the_library {
                        assert_eq!(
                            first_data.and_then(|d| d.attribute("id")),
                            Some("SceStaticInstalled"),
                            "{name}: the library is the document's first <data>"
                        );
                    }
                    // A lowered document is an ecmascript one, and lowering
                    // that again changes nothing.
                    assert_eq!(
                        &lower_source(document, &name).expect("lowers again"),
                        document,
                        "{name}: lowering is idempotent"
                    );
                }
                lowered.push(name);
            }
            Err(refusal) => {
                let text = format!("{refusal:?}");
                assert!(
                    text.contains("UnsupportedFeature"),
                    "{name}: a document it cannot lower is refused as an unsupported feature, \
                     not by another error: {text}"
                );
                refused.push((name, text));
            }
        }
    }
    // Floor: a scan that found nothing to lower would pass. What is lowered
    // today — scalars, checked integers, the typed payload, lists, records,
    // calls of scalar algorithms, a `<send>`'s `<param>`s, a `<final>`'s
    // `<donedata>`, a child session written inline, a hybrid `<invoke>`
    // with the candidates it may start, and an invocation a host runs — is at
    // least these forty-one machines, and the floor rises as the lowering grows.
    assert!(
        lowered.len() >= 41,
        "lowered {lowered:?}, refused {:?}",
        refused.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    // The pairs of a `<send>` and of a `<donedata>` are expressions the walk
    // lowers, in the attribute each is written in, and the Interpreter's own
    // elements read them once when they run: so the fixtures that send
    // themselves their `<param>`s or hand their done event its pairs lower, an
    // enum value among them as the name its enum declares, one that carries on
    // what the event it is on carried, and one that names the variables it sends
    // in a `namelist` or names a record in a `<content expr>`, a `<send>`'s or a
    // `<donedata>`'s, or computes its delay in a `delayexpr` or the name of its
    // event in an `eventexpr`, or the id of the send a `<cancel>` removes in a
    // `sendidexpr`, or the variable a `<send>`'s generated id is written to in
    // an `idlocation`, which the Interpreter reads itself, or the route a
    // `targetexpr` computes among the `sce:targets` the document declares, which
    // the lowering holds the value to, or the processor a `typeexpr` computes
    // among its `sce:types`, held to the same way; a child that declares `<sce:action>`s,
    // inline or as a candidate of a hybrid `<invoke>`, which the engine gives the
    // host its parent's host answers; and children that send to Mesh peers, inline
    // and as a candidate with a child of its own, which the engine gives the router
    // its parent's factory answers (SCE_MESH.md, Child Sessions in Mesh).
    for name in [
        "static_child_mesh",
        "static_send_params",
        "static_send_namelist",
        "static_send_content",
        "static_send_delay",
        "static_send_event",
        "static_send_target",
        "static_send_type",
        "static_send_idlocation",
        "static_child_host",
        "static_child_host_hybrid",
        "static_cancel_expr",
        "static_wire_enum",
        // ... and the conversions between a value of an enum and the number its
        // variant declares, which read the library's table (docs/adr/0047).
        "static_enum_number",
        "static_payload_relay",
        "static_donedata",
        "static_donedata_content",
        "static_donedata_record",
        "static_invoke",
        "static_invoke_params",
        "static_invoke_string",
        "static_invoke_optional",
        "static_invoke_hybrid",
        "static_invoke_hybrid_saved",
        // ... and a child named by `src`, here the standard shared record list
        // (`sce:std/shared/record_list.scxml`), lowered from the library and
        // written beside its parent.
        "record_list_parent",
    ] {
        assert!(
            lowered.iter().any(|lowered_name| lowered_name == name),
            "{name} is lowered for the Interpreter: lowered {lowered:?}"
        );
    }
    // An invocation a host runs is served by the invoker the host registered on the
    // machine (docs/adr/0084, D4 and D7), so the four machines of the standard host
    // features lower: their requests are lowered as a `<send>`'s pairs are, and the
    // element is the author's.
    for name in [
        "static_host_clock",
        "static_host_http",
        "static_host_kv",
        "static_host_notify",
    ] {
        assert!(
            lowered.iter().any(|lowered_name| lowered_name == name),
            "{name} is lowered for the Interpreter: lowered {lowered:?}"
        );
    }
    // A host operation is performed by the host installed on the machine
    // (`INativeActionHost`), so `static_host_call` lowers: its arguments are
    // expressions, lowered as any are. So does the tree whose parent calls its
    // host inside a transition while its inline child runs a host invocation
    // (docs/adr/0084, D2 as amended at step 10), which the Interpreter's inbox
    // test runs from the document `sce-codegen lower` makes of it.
    for name in ["static_host_call", "static_invoke_ending_order"] {
        assert!(
            lowered.iter().any(|lowered_name| lowered_name == name),
            "{name} is lowered for the Interpreter: lowered {lowered:?}"
        );
    }
    // An action states its effect, and the Interpreter reads `effect` and
    // `released-by` as the generated engines do (docs/adr/0038), so the machine
    // that states a duty with its release lowers too: the two attributes travel
    // in the document as the author wrote them.
    assert!(
        lowered.iter().any(|name| name == "static_host_duty"),
        "static_host_duty is lowered for the Interpreter: lowered {lowered:?}"
    );
    let duty = lowered_fixture("static_host_duty");
    assert!(
        duty.contains(
            r#"<sce:action name="start_keepalive" effect="duty" released-by="stop_keepalive"/>"#
        ),
        "the effect and the release are carried as written: {duty}"
    );
    // What is refused says which construct, so an author knows what to change.
    for (name, text) in &refused {
        assert!(
            text.contains("has no ecmascript lowering"),
            "{name}: the refusal names the construct: {text}"
        );
    }
}

fn lowered_fixture(machine: &str) -> String {
    let path = fixtures().join(format!("{machine}.scxml"));
    lower_file(path.to_str().unwrap(), Vec::new())
        .unwrap_or_else(|e| panic!("{machine} does not lower: {e:?}"))
}

/// A record is declared by one `<data>` built whole from its fields, read as
/// the author wrote it, and written a field at a time as an `<assign>` of the
/// whole record — through the library, which returns the record changed in that
/// field and leaves the one it was given alone.
#[test]
fn a_record_is_built_whole_and_written_a_field_at_a_time() {
    let out = lowered_fixture("static_record_fields");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    // The attributes as a reader decodes them, which is what the script engine
    // is handed: `'` is written `&apos;` in the text of the document.
    let attribute = |name: &str| -> Vec<(String, String)> {
        document
            .descendants()
            .filter(|n| n.is_element())
            .filter_map(|n| {
                n.attribute(name)
                    .map(|v| (n.tag_name().name().to_string(), v.to_string()))
            })
            .collect()
    };
    let expr = attribute("expr");
    let has = |element: &str, value: &str| expr.contains(&(element.to_string(), value.to_string()));
    assert!(
        has("data", "({'year': 2026, 'month': 9, 'dayOfMonth': 24})"),
        "the record is built whole from its fields: {expr:?}"
    );
    assert!(
        has(
            "assign",
            "SceStatic.set(shown, 'dayOfMonth', SceStatic.U8.add(shown.dayOfMonth, 1))"
        ),
        "a field written from the record's own value: {expr:?}"
    );
    assert!(
        has(
            "assign",
            "SceStatic.set(shown, 'year', SceStatic.field(_event.data, 'year', 'uint16'))"
        ),
        "a field replaced from the typed payload: {expr:?}"
    );
    assert!(
        attribute("cond").contains(&(
            "transition".to_string(),
            "shown.dayOfMonth < 28".to_string()
        )),
        "a read of a field, in a guard that cannot fail, is the author's"
    );
    // The `<sce:set>`s are gone as elements — the header comment that names
    // them is the author's, and stays.
    assert!(
        !document
            .descendants()
            .any(|n| n.is_element() && n.tag_name().name() == "set"),
        "{out}"
    );
}

/// A payload's text and byte-string fields are read through the library's
/// `field` with the bound their schema declares, so a delivery past it is
/// refused wherever an expression reads the field, the top level included, and
/// not only where the value is written somewhere narrower (docs/adr/0086,
/// section 4.4). `static_record_string`'s `label` is `string[8]` and
/// `static_payload_bytes`'s `frame` is `bytes[8]`.
#[test]
fn a_payload_text_and_byte_string_are_read_held_to_their_bound() {
    for (machine, read) in [
        (
            "static_record_string",
            "SceStatic.field(_event.data, 'label', {string: 8})",
        ),
        (
            "static_payload_bytes",
            "SceStatic.field(_event.data, 'frame', {bytes: 8})",
        ),
    ] {
        let out = lowered_fixture(machine);
        let document = roxmltree::Document::parse(&out).expect("well-formed");
        let reads: Vec<&str> = document
            .descendants()
            .filter(|n| n.is_element())
            .flat_map(|n| [n.attribute("expr"), n.attribute("cond")])
            .flatten()
            .collect();
        assert!(
            reads.iter().any(|value| value.contains(read)),
            "{machine}: every read of the field holds its bound, `{read}`: {reads:?}"
        );
        let unbounded = read.split(", {").next().unwrap_or(read).to_string() + ", '";
        assert!(
            !reads.iter().any(|value| value.contains(&unbounded)),
            "{machine}: no read of the field is held to no bound: {reads:?}"
        );
    }
}

/// An algorithm the document imports travels in it: installed by the library's
/// `<data>` under `SceStatic.algorithms`, and called by that name from the
/// guard that calls it — the function a generated machine would have linked.
#[test]
fn an_imported_algorithm_travels_in_the_document_and_is_called_by_its_name() {
    let out = lowered_fixture("static_record");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    let installer = document
        .descendants()
        .find(|n| n.attribute("id") == Some("SceStaticInstalled"))
        .and_then(|n| n.attribute("expr"))
        .expect("the library is installed");
    assert!(
        installer.contains(
            "SceStatic.algorithms.month_length = function (year, month) { var days = 31;"
        ),
        "the algorithm is installed beside the library: {installer}"
    );
    let guard = document
        .descendants()
        .filter(|n| n.attribute("event") == Some("next"))
        .find_map(|n| n.attribute("cond"))
        .expect("the guard that calls it");
    assert_eq!(
        guard, "shown.dayOfMonth < SceStatic.algorithms.month_length(shown.year, shown.month)",
        "the call reads the record's fields, and fails as an expression of the document does"
    );

    // A may-fail algorithm keeps its preconditions, and the document that
    // calls it says nothing of how a failure is passed on: it is a throw.
    let sync = lowered_fixture("sync_client");
    assert!(
        sync.contains("SceStatic.require("),
        "a precondition of sync_failure is checked: {sync}"
    );
}

/// A document under another data model is not this lowering's to change.
#[test]
fn a_document_under_another_data_model_is_returned_as_written() {
    let ecmascript = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <datamodel><data id="n" expr="1 + 2"/></datamodel>
  <state id="s"/>
</scxml>"#;
    assert_eq!(lower_source(ecmascript, "plain").unwrap(), ecmascript);
}

/// The text a `<final>`'s `<donedata>` carries is the string it spells on every
/// backend, so it is finished in the document: an engine reads `42` as a number
/// and `"42"` as the string, and the lowered `<content>` is the second.
#[test]
fn a_donedata_text_is_finished_to_the_string_it_spells() {
    let lowered = lowered_fixture("static_donedata_content");
    assert!(
        lowered.contains("<content>&quot;42&quot;</content>"),
        "the text is written as the JSON string it spells: {lowered}"
    );
    // The pairs are expressions the walk lowers where each is written.
    let pairs = lowered_fixture("static_donedata");
    assert!(
        pairs.contains(r#"<param name="total" expr="SceStatic.U32.mul(count, 2)"/>"#),
        "a pair's checked operation is a library call: {pairs}"
    );
}

/// A `<content>` that holds an element is read by an engine as a document, where
/// the generated backends carry it as a string: refused by name, not lowered to
/// something that answers differently.
#[test]
fn a_donedata_content_holding_an_element_is_refused_by_name() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="n" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="go" target="d"/></state>
  <final id="d"><donedata><content><result code="7">finished</result></content></donedata></final>
</scxml>"#;
    let refusal = lower_source(document, "xml_content").expect_err("a document it cannot lower");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("holding an element"),
        "the refusal names the construct: {text}"
    );
}

/// The text of a `<send>`'s inline `<content>` is the string it spells on every
/// backend, as a `<donedata>`'s is, so it is finished in the document: an engine
/// reads `42` as a number and `"42"` as the string. A `<content>` that holds an
/// element is refused by name, for the reason a `<donedata>`'s is.
#[test]
fn a_send_content_is_finished_to_the_string_it_spells() {
    let send = |content: &str| {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="n" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="go" type="internal"><send event="note"><content>{content}</content></send></transition></state>
</scxml>"#
        )
    };
    let lowered = lower_source(&send("42"), "send_text").expect("a text content lowers");
    assert!(
        lowered.contains("<content>&quot;42&quot;</content>"),
        "the text is written as the JSON string it spells: {lowered}"
    );
    let refusal = lower_source(&send(r#"<item code="7">ready</item>"#), "send_xml")
        .expect_err("a content holding an element");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("holds an element"),
        "the refusal names the construct: {text}"
    );
}

/// A `<send>`'s `<content expr>` that names one value is the event's whole data,
/// which the Interpreter's own `<send>` evaluates when it runs: the expression is
/// lowered in the attribute it is written in, and stays a `<content expr>` — no
/// pair is made of it, as one is of a record's fields.
#[test]
fn a_send_content_that_names_a_value_is_lowered_in_its_attribute() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="n" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="go" type="internal"><send event="note"><content expr="n + 1"/></send></transition></state>
</scxml>"#;
    let lowered =
        lower_source(document, "send_value").expect("a content that names a value lowers");
    assert!(
        lowered.contains(r#"<content expr=""#),
        "the value is still the content's expression: {lowered}"
    );
    assert!(
        !lowered.contains("<param"),
        "a value is not made into a pair: {lowered}"
    );
    assert!(
        lowered.contains(r#"datamodel="ecmascript""#),
        "the document is the ecmascript one the Interpreter runs: {lowered}"
    );
}

/// The same for a `<final>`'s `<donedata>`: the Interpreter's own element
/// evaluates the `<content expr>` when the state is entered, so the expression is
/// lowered in the attribute it is written in and no pair is made of it.
#[test]
fn a_donedata_content_that_names_a_value_is_lowered_in_its_attribute() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="n" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="go" target="d"/></state>
  <final id="d"><donedata><content expr="n + 1"/></donedata></final>
</scxml>"#;
    let lowered =
        lower_source(document, "donedata_value").expect("a content that names a value lowers");
    assert!(
        lowered.contains(r#"<content expr=""#),
        "the value is still the content's expression: {lowered}"
    );
    assert!(
        !lowered.contains("<param"),
        "a value is not made into a pair: {lowered}"
    );
}

/// An invocation a host runs keeps its element, typed request and result
/// included, and its `<param>`s are lowered in the attributes they are written
/// in (docs/adr/0084, D7): a 64-bit integer crosses as the text of its digits, as
/// a `<send>`'s does, and an operation the model checks is the library's call, so
/// the Interpreter computes what the generated backends compute.
#[test]
fn a_host_run_invoke_keeps_its_element_and_lowers_its_params() {
    let lowered = lowered_fixture("static_host_notify");
    let parsed = roxmltree::Document::parse(&lowered).expect("well-formed");
    let invoke = |id: &str| {
        parsed
            .descendants()
            .find(|n| {
                n.is_element() && n.tag_name().name() == "invoke" && n.attribute("id") == Some(id)
            })
            .unwrap_or_else(|| panic!("the <invoke id=\"{id}\"> stands: {lowered}"))
    };
    let param = |id: &str, name: &str| -> String {
        invoke(id)
            .children()
            .find(|n| n.is_element() && n.attribute("name") == Some(name))
            .and_then(|n| n.attribute("expr"))
            .unwrap_or_else(|| panic!("<invoke id=\"{id}\"> keeps <param name=\"{name}\">"))
            .to_string()
    };
    let post = invoke("post");
    assert_eq!(post.attribute("type"), Some("sce-host-notify-post"));
    assert_eq!(
        post.attribute(("http://sce.dev/ext", "request")),
        Some("NotifyPostRequest"),
        "the typed request travels as written"
    );
    assert_eq!(param("post", "id"), "id", "a string is read as it is held");
    let at = param("post", "at_ms");
    assert!(
        at.starts_with("String(") && at.contains("at_ms"),
        "a 64-bit integer crosses as the text of its digits: {at}"
    );
    let far = param("far", "at_ms");
    assert!(
        far.starts_with("String(") && far.contains("SceStatic.") && !far.contains("at_ms + 1"),
        "the sum the model checks is the library's, carried as its digits: {far}"
    );
}

/// A model's generation is the one host-run type the lowering refuses
/// (docs/adr/0087, section 7.1): its request's `answer_schema` and the table it
/// is resolved in are written by the build, and the lowered document would
/// carry neither. Refused by name, naming the type and its state.
#[test]
fn a_model_generation_is_refused_by_name() {
    let path = fixtures().join("static_host_model.scxml");
    let refusal = lower_file_set(path.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature")
            && text.contains("a model's generation")
            && text.contains("sce-host-model-generate"),
        "the refusal names the model's generation: {text}"
    );
}

/// The reference harness (docs/adr/0087, sections 6 and 7.1) asks its host's model
/// through the same host-run type, so the Interpreter refuses the harness by the
/// name of its generation, naming the type and the state that holds the call: the
/// refusal is the one section 7.1 states, and no other construct of the harness
/// (its templates, its lift, its enum conversions, its bound read by name) is what
/// stands in the way.
#[test]
fn the_reference_harness_is_refused_by_the_name_of_its_generation() {
    let path = fixtures().join("static_model_harness.scxml");
    let refusal = lower_file_set(path.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature")
            && text.contains("a model's generation")
            && text.contains("sce-host-model-generate")
            && text.contains("`generate`"),
        "the refusal names the harness's generation and its state: {text}"
    );
}

/// An inline child is a `sce-static` document of its own, lowered where it stands
/// in its parent's text, and a string it bounds is handed through that bound: the
/// Interpreter holds no bound, so the lowered `<param>` carries it.
#[test]
fn an_inline_child_is_lowered_where_it_stands_and_its_bound_is_carried() {
    let lowered = lowered_fixture("static_invoke_string");
    assert_eq!(
        lowered
            .matches(r#"initial="waiting" datamodel="ecmascript""#)
            .count(),
        3,
        "each of the three children is an ecmascript document: {lowered}"
    );
    assert!(
        lowered.contains(r#"<param name="title" expr="SceStatic.bounded(lengthy, 4)"/>"#),
        "a string the child bounds is handed through the bound: {lowered}"
    );
}

/// A child's variable of `optional<T>` is handed the whole optional value its
/// `<param>` computes (docs/adr/0055, step 10): an optional value as it is, held
/// through the bound the child declares for its text; a value of `T` as the value,
/// which the Interpreter's optional is, held through the same bound; and `??` as
/// the library's call.
#[test]
fn a_childs_optional_variable_is_handed_the_optional_value_and_its_bound() {
    let lowered = lowered_fixture("static_invoke_optional");
    assert_eq!(
        lowered
            .matches(r#"initial="waiting" datamodel="ecmascript""#)
            .count(),
        6,
        "each of the six children is an ecmascript document: {lowered}"
    );
    assert!(
        lowered.contains(r#"<param name="label" expr="SceStatic.bounded(lengthy, 4)"/>"#),
        "a text of the parent's handed to a child's optional text is held to its bound: {lowered}"
    );
    assert!(
        lowered.contains("SceStatic.mapPresent(label, "),
        "an optional text is handed with its present value held to the child's bound: {lowered}"
    );
    assert!(
        lowered.contains("SceStatic.otherwise(size, "),
        "`size ?? 0` is the library's call: {lowered}"
    );
}

/// A `sce-static` statechart whose one state holds `invokes`, with a string
/// `pick` a hybrid `<invoke>` among them may name its candidate by.
fn parent_starting(invokes: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel>
    <data id="n" sce:type="uint32" expr="0"/>
    <data id="pick" sce:type="string" sce:capacity="32" expr="'child.scxml'"/>
  </datamodel>
  <state id="s">{invokes}</state>
</scxml>"#
    )
}

/// A `sce-static` child that adds to a counter it holds, so its lowering calls
/// the library: `SceStatic.U8.add(k, 1)`.
fn counting_child() -> String {
    r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="a">
  <datamodel><data id="k" sce:type="uint8" expr="0"/></datamodel>
  <state id="a"><transition event="go" target="a"><assign location="k" expr="k + 1"/></transition></state>
</scxml>"#
        .to_string()
}

/// The `src` of every `<invoke>` of `document`, in document order.
fn invoke_srcs(document: &str) -> Vec<String> {
    let parsed = roxmltree::Document::parse(document).expect("well-formed");
    parsed
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "invoke")
        .filter_map(|n| n.attribute("src").map(str::to_string))
        .collect()
}

/// The names of a lowered set, the document asked for first.
fn names_of(set: &[LoweredDocument]) -> Vec<&str> {
    set.iter().map(|d| d.file_name.as_str()).collect()
}

/// A child named by `src` has no place in its parent's text, and the Interpreter
/// loads it from beside the invoking document: it is lowered as a document of its
/// own and written beside the parent under the name the `src` says, which is left
/// as the author wrote it. The single-document entry refuses the pair, naming the
/// way to ask for the set.
#[test]
fn a_child_named_by_src_is_lowered_beside_its_parent() {
    let (_dir, parent) = written(&[
        (
            "parent.scxml",
            parent_starting(r#"<invoke type="scxml" id="c" src="child.scxml"/>"#),
        ),
        ("child.scxml", counting_child()),
    ]);
    let set = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect("lowers");
    assert_eq!(names_of(&set), ["parent.scxml", "child.scxml"]);
    assert_eq!(
        invoke_srcs(&set[0].text),
        ["child.scxml"],
        "a `src` that names the file the child is written as is the author's: {}",
        set[0].text
    );
    assert!(
        set[1].text.contains(r#"datamodel="ecmascript""#)
            && set[1].text.contains("SceStatic.U8.add(k, 1)"),
        "the child is lowered as a document of its own: {}",
        set[1].text
    );

    let refusal = lower_file(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature")
            && text.contains("by `src`")
            && text.contains("--out-dir"),
        "the refusal names the child and the option: {text}"
    );
}

/// A statechart of the standard library has no file: it is lowered from the text
/// the generator embeds, written under its own stem, and the parent's `src`, a
/// library name the Interpreter could not load, becomes that file's name. Nothing
/// stands beside the parent for it, so a set held in memory needs no entry.
#[test]
fn a_standard_child_is_lowered_from_the_library_under_its_stem() {
    let held = std::collections::BTreeMap::from([(
        "keeper.scxml".to_string(),
        parent_starting(
            r#"<invoke type="scxml" id="list" src="sce:std/shared/record_list.scxml"/>"#,
        ),
    )]);
    let set = lower_documents("keeper.scxml", &held).expect("lowers");
    assert_eq!(names_of(&set), ["keeper.scxml", "record_list.scxml"]);
    assert_eq!(
        invoke_srcs(&set[0].text),
        ["record_list.scxml"],
        "the library name is rewritten to the file the child is written as: {}",
        set[0].text
    );
    assert!(
        set[1].text.contains(r#"datamodel="ecmascript""#),
        "the standard child is lowered as a document of its own"
    );
}

/// The set is every document a lowered one may start, at every depth: a child
/// named by `src` brings the child it names by `src` in turn.
#[test]
fn a_child_named_by_src_brings_the_children_it_names_in_turn() {
    let (_dir, parent) = written(&[
        (
            "parent.scxml",
            parent_starting(r#"<invoke type="scxml" id="c" src="middle.scxml"/>"#),
        ),
        (
            "middle.scxml",
            parent_starting(r#"<invoke type="scxml" id="c" src="leaf.scxml"/>"#),
        ),
        ("leaf.scxml", counting_child()),
    ]);
    let set = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect("lowers");
    assert_eq!(
        names_of(&set),
        ["parent.scxml", "middle.scxml", "leaf.scxml"]
    );
    for document in &set {
        assert!(
            document.text.contains(r#"datamodel="ecmascript""#),
            "{}: every document of the set is lowered",
            document.file_name
        );
    }
}

/// One document reached by two `<invoke>`s by `src` — one spelled with a `file:`
/// scheme — and as the candidate of a hybrid one is lowered once, and every
/// `src` names the one file it is written as.
#[test]
fn a_child_reached_twice_is_lowered_once() {
    let (_dir, parent) = written(&[
        (
            "parent.scxml",
            parent_starting(
                r#"<invoke type="scxml" id="c" src="child.scxml"/><invoke type="scxml" id="d" src="file:child.scxml"/><invoke type="scxml" id="h" srcexpr="pick" sce:candidates="child.scxml"/>"#,
            ),
        ),
        ("child.scxml", counting_child()),
    ]);
    let set = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect("lowers");
    assert_eq!(names_of(&set), ["parent.scxml", "child.scxml"]);
    assert_eq!(
        invoke_srcs(&set[0].text),
        ["child.scxml", "child.scxml"],
        "the `file:` spelling is rewritten to the file's name: {}",
        set[0].text
    );
}

/// A child is written under its stem and started by that name, so two documents
/// of one stem — a parent's child, and the child a candidate in another directory
/// names by the same `src` — cannot stand in one directory: refused, naming the
/// stem.
#[test]
fn two_documents_started_under_one_stem_are_refused_by_name() {
    let (_dir, parent) = written(&[
        (
            "parent.scxml",
            parent_starting(
                r#"<invoke type="scxml" id="h" srcexpr="pick" sce:candidates="sub/middle.scxml"/><invoke type="scxml" id="c" src="child.scxml"/>"#,
            ),
        ),
        ("child.scxml", counting_child()),
        (
            "sub/middle.scxml",
            parent_starting(r#"<invoke type="scxml" id="c" src="child.scxml"/>"#),
        ),
        ("sub/child.scxml", counting_child()),
    ]);
    let refusal = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("the one stem `child`"),
        "the refusal names the stem: {text}"
    );
}

/// A `src` that names a Mesh peer rather than a document has no document to
/// lower beside its parent: refused by name.
#[test]
fn a_src_that_names_a_peer_is_refused_by_name() {
    let (_dir, parent) = written(&[(
        "parent.scxml",
        parent_starting(r##"<invoke type="scxml" id="c" src="#peer"/>"##),
    )]);
    let refusal = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature")
            && text.contains("`#peer`")
            && text.contains("names no document"),
        "the refusal names the `src`: {text}"
    );
}

/// A `srcexpr` names the child by a value, which is a hybrid `<invoke>`: with no
/// `sce:candidates` there is no finite set of documents to lower, and the
/// document is refused where it is read, naming the attribute to declare.
#[test]
fn a_srcexpr_with_no_candidates_is_refused_by_name() {
    let (_dir, parent) = written(&[(
        "parent.scxml",
        parent_starting(r#"<invoke type="scxml" id="c" srcexpr="pick"/>"#),
    )]);
    let refusal = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("sce:candidates"),
        "the refusal names the attribute: {text}"
    );
}

/// A document lowered from memory alone has nothing beside it, so one whose
/// `<invoke>` starts a document of its own is refused by name rather than lowered
/// into a parent that starts a file nobody wrote.
#[test]
fn a_document_from_memory_that_starts_another_is_refused_by_name() {
    let refusal = lower_source(
        &parent_starting(r#"<invoke type="scxml" id="c" src="child.scxml"/>"#),
        "alone",
    )
    .expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("`child`"),
        "the refusal names the document it starts: {text}"
    );
}

/// A string's bound a deployment may move (`sce:limit`) is held by every
/// expression that writes the string, at the number the build gave the name.
/// The attributes that declare it stay as the author wrote them and are carried
/// by nothing more in an ecmascript document, the name as the bound it names is,
/// so the lowered document reads again and lowers to itself.
#[test]
fn a_named_bound_is_held_by_the_expressions_and_the_lowered_document_reads_again() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel>
    <data id="long" sce:type="string" sce:capacity="16" expr="'abcdef'"/>
    <data id="named" sce:type="string" sce:capacity="4" sce:limit="named_text" expr="''"/>
  </datamodel>
  <state id="s"><transition event="go" type="internal"><assign location="named" expr="long"/></transition></state>
</scxml>"#;
    let limits = sce_build::limits::Limits::none()
        .with("named_text", 8)
        .expect("a limit");
    let _scope = sce_build::limits::scope(&limits);
    let lowered = lower_source(document, "named").expect("lowers");
    let parsed = roxmltree::Document::parse(&lowered).expect("well-formed");
    let assigned: Vec<&str> = parsed
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "assign")
        .filter_map(|n| n.attribute("expr"))
        .collect();
    assert_eq!(
        assigned,
        ["SceStatic.bounded(long, 8)"],
        "the bound is the number the build gave the name: {lowered}"
    );
    assert!(
        lowered.contains(r#"sce:limit="named_text""#),
        "the author's attributes are kept: {lowered}"
    );
    assert_eq!(
        lower_source(&lowered, "named").expect("the lowered document reads again"),
        lowered,
        "lowering is idempotent"
    );
}

/// A string a `namelist` hands a child is handed as the Interpreter holds it,
/// with no expression to carry the bound the child declared for it, so the
/// bound would go unkept: refused by name, where a `<param>` carries it.
#[test]
fn a_namelist_name_that_hands_a_child_a_bounded_string_is_refused_by_name() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel><data id="title" sce:type="string" sce:capacity="8" expr="'abc'"/></datamodel>
  <state id="s">
    <invoke type="scxml" id="c" namelist="title">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="w" datamodel="sce-static">
          <datamodel><data id="title" sce:type="string" sce:capacity="4" expr="'ab'"/></datamodel>
          <state id="w"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;
    let refusal = lower_source(document, "namelist").expect_err("a document it cannot lower");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("namelist"),
        "the refusal names the construct: {text}"
    );
}

/// The `<invoke>` of a hybrid parent: `srcexpr="pick"` among `candidates`, with
/// `attributes` beside them and `params` inside.
fn hybrid_parent(candidates: &str, attributes: &str, params: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="s">
  <datamodel>
    <data id="pick" sce:type="string" sce:capacity="32" expr="'a.scxml'"/>
    <data id="text" sce:type="string" sce:capacity="16" expr="'abc'"/>
  </datamodel>
  <state id="s">
    <invoke type="scxml" id="c" srcexpr="pick" sce:candidates="{candidates}" {attributes}>{params}</invoke>
  </state>
</scxml>"#
    )
}

/// A candidate that declares the string `t`, bounded to `capacity`.
fn candidate_holding(capacity: u32) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="sce-static" initial="w">
  <datamodel><data id="t" sce:type="string" sce:capacity="{capacity}" expr="'ab'"/></datamodel>
  <state id="w"/>
</scxml>"#
    )
}

/// `files` written into a fresh directory, and the path of the first.
fn written(files: &[(&str, String)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    for (name, text) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).expect("a directory");
        std::fs::write(path, text).expect("a document is written");
    }
    let first = dir.path().join(files[0].0);
    (dir, first)
}

/// A hybrid `<invoke>` starts a document of its own, which the Interpreter loads
/// at run time from beside the invoking one: the lowering is the invoking
/// document and each candidate, lowered, and the value is reduced to the stem of
/// the one it names by the library, which answers the name that candidate is
/// written under.
#[test]
fn a_hybrid_invoke_is_lowered_with_the_candidates_it_may_start() {
    let path = fixtures().join("static_invoke_hybrid.scxml");
    let set = lower_file_set(path.to_str().unwrap(), Vec::new()).expect("lowers");
    let names: Vec<&str> = set.iter().map(|d| d.file_name.as_str()).collect();
    assert_eq!(
        names,
        [
            "static_invoke_hybrid.scxml",
            "static_hybrid_first.scxml",
            "static_hybrid_second.scxml"
        ],
        "the document asked for, then each candidate under its stem"
    );
    let parent = &set[0].text;
    assert_eq!(
        parent
            .matches(
                r#"srcexpr="SceStatic.candidate(pick, [&quot;static_hybrid_first&quot;, &quot;static_hybrid_second&quot;])""#
            )
            .count(),
        4,
        "each of the four invokes names its candidate through the library: {parent}"
    );
    for candidate in &set[1..] {
        assert!(
            candidate.text.contains(r#"datamodel="ecmascript""#),
            "{}: a candidate is lowered as a document of its own",
            candidate.file_name
        );
    }
}

/// The documents it would have to start are other files, so the single-document
/// entry refuses one, naming the way to ask for the set.
#[test]
fn a_hybrid_invoke_is_refused_where_one_document_is_asked_for() {
    let path = fixtures().join("static_invoke_hybrid.scxml");
    let refusal = lower_file(path.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("--out-dir"),
        "the refusal names the option: {text}"
    );
}

/// The Interpreter carries one expression in an attribute, and an argument is
/// handed to every candidate: two candidates that take it as strings bounded
/// differently would need two expressions for the one written, so it is refused,
/// naming it.
#[test]
fn an_argument_the_candidates_bound_differently_is_refused_by_name() {
    let params = r#"<param name="t" expr="text"/>"#;
    let (_dir, parent) = written(&[
        ("parent.scxml", hybrid_parent("a.scxml b.scxml", "", params)),
        ("a.scxml", candidate_holding(4)),
        ("b.scxml", candidate_holding(8)),
    ]);
    let refusal = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("`text`") && text.contains("two ways"),
        "the refusal names the argument: {text}"
    );

    // Bounded alike, the one expression serves both.
    let (_dir, parent) = written(&[
        ("parent.scxml", hybrid_parent("a.scxml b.scxml", "", params)),
        ("a.scxml", candidate_holding(4)),
        ("b.scxml", candidate_holding(4)),
    ]);
    let set = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect("lowers");
    assert!(
        set[0]
            .text
            .contains(r#"<param name="t" expr="SceStatic.bounded(text, 4)"/>"#),
        "{}",
        set[0].text
    );
}

/// A `namelist` name is carried as the Interpreter holds it, with no expression
/// to carry a bound: one that lands in a string a candidate bounds is refused,
/// naming the candidate.
#[test]
fn a_namelist_name_a_candidate_bounds_is_refused_by_name() {
    let (_dir, parent) = written(&[
        (
            "parent.scxml",
            hybrid_parent("a.scxml b.scxml", r#"namelist="t""#, "")
                .replace(r#"<data id="text""#, r#"<data id="t""#),
        ),
        ("a.scxml", candidate_holding(4)),
        ("b.scxml", candidate_holding(4)),
    ]);
    let refusal = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature")
            && text.contains("namelist")
            && text.contains("hybrid")
            && text.contains("`a`"),
        "the refusal names the construct and the candidate: {text}"
    );
}

/// A candidate is written under its stem and a value is matched by it, so two
/// candidates of one stem — even one a candidate names in turn — cannot stand in
/// one directory.
#[test]
fn two_candidates_of_one_stem_are_refused_by_name() {
    let (_dir, parent) = written(&[
        ("parent.scxml", hybrid_parent("a.scxml b.scxml", "", "")),
        ("a.scxml", candidate_holding(4)),
        ("b.scxml", hybrid_parent("sub/a.scxml", "", "")),
        ("sub/a.scxml", candidate_holding(4)),
    ]);
    let refusal = lower_file_set(parent.to_str().unwrap(), Vec::new()).expect_err("a refusal");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("UnsupportedFeature") && text.contains("the one stem `a`"),
        "the refusal names the stem: {text}"
    );
}

/// A caller with no file system hands the documents over, and gets the lowering
/// it would get from files: for every fixture, the set lowered from memory is the
/// set lowered from the directory, byte for byte, and a fixture the directory
/// refuses is refused from memory. The imported algorithms, the event-schemas and
/// the candidates of a hybrid `<invoke>` all stand beside the fixtures, so none of
/// them is a case this leaves out.
#[test]
fn documents_held_in_memory_lower_as_the_files_do() {
    let mut held = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(fixtures()).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|ext| ext == "scxml") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            held.insert(name, std::fs::read_to_string(&path).expect("a fixture"));
        }
    }
    let mut lowered_from_both = 0;
    for statechart in static_statecharts() {
        let name = statechart
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let from_files = lower_file_set(statechart.to_str().unwrap(), Vec::new());
        let from_memory = lower_documents(&name, &held);
        match (from_files, from_memory) {
            (Ok(files), Ok(memory)) => {
                assert_eq!(files, memory, "{name}: lowered from memory as from files");
                lowered_from_both += 1;
            }
            (Err(_), Err(_)) => {}
            (files, memory) => panic!(
                "{name}: files give {:?} and memory gives {:?}",
                files.map(|set| set.len()),
                memory.map(|set| set.len())
            ),
        }
    }
    assert!(
        lowered_from_both >= 40,
        "the comparison covered {lowered_from_both} fixtures: the scan is broken, not the lowering"
    );
}

/// A name the set lacks is a file that is not there, and not a read of the disk:
/// a set holding a statechart without the algorithm it imports refuses it.
#[test]
fn a_document_the_set_lacks_is_a_file_that_is_not_there() {
    let text = std::fs::read_to_string(fixtures().join("static_payload.scxml")).expect("a fixture");
    let held = std::collections::BTreeMap::from([("static_payload.scxml".to_string(), text)]);
    // `schema_day.scxml` stands beside it on disk, and is not in the set.
    assert!(fixtures().join("schema_day.scxml").exists());
    lower_documents("static_payload.scxml", &held).expect_err("its schema is not in the set");

    // The standard library is built in, so a statechart that imports only that
    // needs nothing beside it.
    let text = std::fs::read_to_string(fixtures().join("sync_client.scxml")).expect("a fixture");
    let held = std::collections::BTreeMap::from([("sync_client.scxml".to_string(), text)]);
    lower_documents("sync_client.scxml", &held).expect("the standard library needs no entry");
}

/// A bounded-collection is a kind of its own, run by every generated backend,
/// but a `sce-static` statechart has no variable of it, so no statechart could
/// hand the Interpreter's copy of an algorithm one to loop over. Lowering that
/// loop would be code nothing reaches, so it is refused, and the refusal says
/// why. The day a statechart can hold one, these three are the algorithms to
/// lower with it.
#[test]
fn an_algorithm_looping_over_a_bounded_collection_is_refused_by_name() {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    for name in [
        "algorithm_keyexpr_match_first",
        "algorithm_bc_iter_minimal",
        "algorithm_keyexpr_field_match",
    ] {
        let path = resources.join(format!("{name}.scxml"));
        let refusal = sce_build::forge::static_js::lower_algorithm_document(path.to_str().unwrap())
            .expect_err("a loop over a bounded-collection has no lowering");
        let text = format!("{refusal:?}");
        assert!(
            text.contains("UnsupportedFeature")
                && text.contains("a loop over a bounded-collection import"),
            "{name}: the refusal names the construct and the reason: {text}"
        );
    }
}
