// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A statechart sends a composed value by the name of the place that holds it
//! (docs/adr/0086, section 5.2): a `<param>` that gives a composed field of the
//! schema the document imports for the event names a variable, a field of a
//! record variable, or a part of the payload of the event the transition is
//! on, and is accepted when the place's type fits the field's (section 5.1).
//! Each backend whose row of the one table of what is lowered sends a
//! composed value (`ComposedLowering::of`) writes it by walking its type
//! (section 5.3).
//!
//! # What is held
//!
//! * `check -l <backend>` takes `static_composed_send`, which forwards four
//!   lists and an optional list of the payload and sends a list variable, for
//!   each backend whose row sends a composed value, and every other backend
//!   refuses it by its own name: the backends are asked of the table, never
//!   listed here.
//! * The Rust machine writes each value into the runtime's `ScriptValue` by
//!   one case per constructor: a list as an `Array` of its elements, a record
//!   as an `Object` of its fields, an absent value as `Null`, a 64-bit integer
//!   as the text of its digits, a byte string as its Latin-1 text and an enum
//!   value as its declared name, as a scalar param crosses.
//! * The Kotlin machine writes each into the value its `<send>` holds params
//!   in: a list as a `List`, a record as a `Map`, an absent value as `null`,
//!   and each scalar as a scalar param crosses.
//! * The Python machine writes each into the `ScriptValue` its `<send>` holds
//!   params in, made of a `list`, a `dict` and `None`, each scalar as a scalar
//!   param crosses.
//! * A value whose type may not fit the field's is refused where it is
//!   written, naming the field, the schema that reads it and the place.
//! * A value that names no place is refused.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::composed_lowering::checked_backends;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents a sender of `batch.sent` or `counted.sent` imports, copied
/// beside it.
const SIBLINGS: [&str; 4] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
    "event_counted.scxml",
];

/// The machine of `static_composed_send` for `language`, generated where the
/// fixture is: the file whose name ends in `suffix`.
fn generated_sender(language: sce_build::generator::Language, suffix: &str) -> String {
    let path = fixtures().join("static_composed_send.scxml");
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} sends a composed value: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with(suffix))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the machine's {language:?} is generated"))
}

/// The Rust machine of `static_composed_send`.
fn sender_machine() -> String {
    generated_sender(
        sce_build::generator::Language::Rust,
        "static_composed_send_sm.rs",
    )
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// `sce-codegen <args> <file>` over `document` written beside the batch
/// schemas as `batch_sender.scxml`, and whether it succeeded, with what it
/// said.
fn run(args: &[&str], document: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join("batch_sender.scxml");
    std::fs::write(&path, document).expect("write the document");
    run_on(args, &path)
}

/// `sce-codegen <args> <path>`, and whether it succeeded, with what it said.
fn run_on(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// A statechart that holds the variables `data` and, on each `batch.sent`,
/// sends `batch.sent` to itself with `params`, after a param for each composed
/// field of the schema's first revision, which every delivery carries
/// (docs/adr/0086, section 10.1), each the received event's own (a variable
/// cannot hold a list of records that hold lists, so `static_composed_send`
/// passes them on the same way).
fn sender(data: &str, params: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_sender">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_signal_level.scxml" as="SignalLevel"/>
  <datamodel>
    {data}
  </datamodel>
  <state id="idle">
    <transition event="batch.sent" target="idle">
      <send event="batch.sent" target="#_internal">
        <param name="items" expr="_event.data.items"/>
        <param name="grid" expr="_event.data.grid"/>
        <param name="levels" expr="_event.data.levels"/>
        <param name="maybe" expr="_event.data.maybe"/>
        <param name="spare" expr="_event.data.spare"/>
        {params}
      </send>
    </transition>
  </state>
</scxml>
"##
    )
}

/// Pinned by `ComposedLowering::of`'s `sends` and the gate's `<send>` arm
/// (`sce-build/src/forge/event_schema_check.rs`) and by
/// `Judge::composed_param` (`sce-build/src/forge/static_datamodel.rs`): each
/// backend whose row sends a composed value takes the fixture, and every other
/// backend refuses it by its own name.
#[test]
fn each_backend_that_sends_takes_a_send_of_composed_values_that_fit() {
    for (language, row) in checked_backends() {
        let (ok, out) = run_on(
            &["check", "-l", language],
            &fixtures().join("static_composed_send.scxml"),
        );
        if row.sends {
            assert!(ok, "{language} sends a composed value that fits:\n{out}");
        } else {
            assert!(!ok, "{language} sends no composed value yet:\n{out}");
            assert!(
                out.contains("generate/unsupported-feature"),
                "{language}: {out}"
            );
            assert!(
                out.contains(&format!("has no {language} lowering")),
                "{language}: {out}"
            );
        }
    }
}

/// Pinned by `KotlinComposed`'s writers under `KotlinCarrier::Wire`
/// (`sce-build/src/forge/composed_lowering/kotlin.rs`), `KotlinTarget::composed_send_spelling`
/// and `composed_wire_value` (`sce-build/src/forge/static_lowering.rs`): each part
/// of the payload is written from the place it is read at into the value a Kotlin
/// `<send>` holds its params in, a list as a `List` and a record as a `Map`, which
/// the engine writes as the event's JSON.
#[test]
fn each_value_is_written_by_walking_its_type_on_kotlin() {
    let code = generated_sender(sce_build::generator::Language::Kotlin, "Sm.kt");
    let p = "com.sce.runtime.EventPayload";
    holds(
        &code,
        &[
            &format!(
                "putParam(sendPayload, \"items\", (pendingBatchSentPayload!!.items).map {{ \
                 mapOf<String, Any?>(\"id\" to (it.id).toLong(), \"amount\" to (it.amount).toString(), \
                 \"label\" to it.label, \"level\" to it.level.declaredName, \"due\" to \
                 ((it.due)?.let {{ (it).toString() }}), \"blob\" to {p}.bytesAsText(it.blob), \
                 \"tags\" to (it.tags).map {{ it }}) }})"
            ),
            "putParam(sendPayload, \"grid\", (pendingBatchSentPayload!!.grid).map { (it).map { (it).toLong() } })",
            "putParam(sendPayload, \"levels\", (pendingBatchSentPayload!!.levels).map { it.declaredName })",
            "putParam(sendPayload, \"maybe\", (pendingBatchSentPayload!!.maybe).map { ((it)?.let { (it).toLong() }) })",
            "putParam(sendPayload, \"spare\", ((pendingBatchSentPayload!!.spare)?.let { (it).map { (it).toLong() } }))",
            "putParam(sendPayload, \"late\", (vLate).map { (it).toLong() })",
        ],
    );
}

/// Pinned by `PythonComposed`'s writers under `PythonCarrier::Wire` and its
/// `send_param` (`sce-build/src/forge/composed_lowering/python.rs`),
/// `PythonTarget::composed_send_spelling` and `composed_wire_value`
/// (`sce-build/src/forge/static_lowering.rs`): each part of the payload is
/// written from the place it is read at into the plain value a Python `<send>`'s
/// `ScriptValue` is made of, a list as a `list` and a record as a `dict`, which
/// the runtime writes as the event's JSON.
#[test]
fn each_value_is_written_by_walking_its_type_on_python() {
    let code = generated_sender(sce_build::generator::Language::Python, "_sm.py");
    let put = "self._put_param(_data, _data_repeats, ";
    let payload = "self._pending_batch_sent_payload";
    holds(
        &code,
        &[
            &format!(
                "{put}\"items\", (_ScriptValue.of([{{\"id\": sce_element.id, \"amount\": \
                 str(sce_element.amount), \"label\": sce_element.label, \"level\": \
                 (sce_element.level).sce_name, \"due\": (lambda sce_element: None if \
                 sce_element is None else str(sce_element))(sce_element.due), \"blob\": \
                 (sce_element.blob).decode(\"latin-1\"), \"tags\": [sce_element for \
                 sce_element in sce_element.tags]}} for sce_element in {payload}.items])).to_python())"
            ),
            &format!(
                "{put}\"grid\", (_ScriptValue.of([[sce_element for sce_element in sce_element] \
                 for sce_element in {payload}.grid])).to_python())"
            ),
            &format!(
                "{put}\"levels\", (_ScriptValue.of([(sce_element).sce_name for sce_element in \
                 {payload}.levels])).to_python())"
            ),
            &format!(
                "{put}\"maybe\", (_ScriptValue.of([(lambda sce_element: None if sce_element is \
                 None else sce_element)(sce_element) for sce_element in \
                 {payload}.maybe])).to_python())"
            ),
            &format!(
                "{put}\"spare\", (_ScriptValue.of((lambda sce_element: None if sce_element is \
                 None else [sce_element for sce_element in sce_element])({payload}.spare))).to_python())"
            ),
            "(_ScriptValue.of([sce_element for sce_element in self.v_late])).to_python())",
        ],
    );
}

/// Pinned by `RustComposed`'s writers under `RustCarrier::ScriptValue`
/// (`sce-build/src/forge/composed_lowering/rust.rs`) and by `composed_wire_value`
/// (`sce-build/src/forge/static_lowering.rs`): each part of the payload is
/// written from the place it is read at, by one case per constructor.
#[test]
fn each_value_is_written_by_walking_its_type() {
    let code = sender_machine();
    let s = "::sce_rust_runtime::ScriptValue";
    let p = "::sce_rust_runtime::event_payload";
    holds(
        &code,
        &[
            &format!("{s}::Array(ev.items.iter().map(|h| {s}::Object(vec![(\"id\".to_string(), {s}::Int(i64::from(h.id))), (\"amount\".to_string(), {s}::String(h.amount.to_string())), (\"label\".to_string(), {s}::String(h.label.to_string())), (\"level\".to_string(), {s}::String(h.level.sce_name().to_string())), "),
            &format!("(\"due\".to_string(), h.due.as_ref().map_or({s}::Null, |h| {s}::String(h.to_string()))), "),
            &format!("(\"blob\".to_string(), {s}::String({p}::bytes_as_payload_text(&h.blob))), "),
            &format!("(\"tags\".to_string(), {s}::Array(h.tags.iter().map(|h| {s}::String(h.to_string())).collect()))].into_iter().collect())).collect())"),
            &format!("{s}::Array(ev.grid.iter().map(|h| {s}::Array(h.iter().map(|h| {s}::Int(i64::from(*h))).collect())).collect())"),
            &format!("{s}::Array(ev.levels.iter().map(|h| {s}::String(h.sce_name().to_string())).collect())"),
            &format!("{s}::Array(ev.maybe.iter().map(|h| h.as_ref().map_or({s}::Null, |h| {s}::Int(i64::from(*h)))).collect())"),
            &format!("ev.spare.as_ref().map_or({s}::Null, |h| {s}::Array(h.iter().map(|h| {s}::Int(i64::from(*h))).collect()))"),
            &format!(".late.iter().map(|h| {s}::Int(i64::from(*h))).collect())"),
        ],
    );
}

/// Pinned by the fit `Judge::composed_param` asks of the place's type and the
/// field's (`sce-build/src/forge/static_datamodel.rs`).
#[test]
fn a_value_that_may_not_fit_is_refused_naming_the_field_and_the_place() {
    let document = sender(
        r#"<data id="wide" sce:type="list&lt;uint8&gt;[3]"/>"#,
        r#"<param name="late" expr="wide"/>"#,
    );
    let (ok, out) = run(&["check", "-l", "rust"], &document);
    assert!(!ok, "a list of 3 may not fit a field of 2:\n{out}");
    assert!(out.contains("scxml/static-datamodel-rule"), "{out}");
    assert!(
        out.contains("the schema `event_batch` this document declares `batch.sent` by reads it as its field `late`"),
        "{out}"
    );
    assert!(out.contains("'wide' is a "), "{out}");
}

/// A statechart that holds the variables `data` and, on `go`, sends
/// `counted.sent` (whose `kept` is a `list<uint8>[1..2]` and `hops` a `uint8`,
/// event_counted.scxml) with `kept` given by `expr` and `hops` by 1.
fn counted_sender(data: &str, expr: &str) -> String {
    counted_send(data, "go", expr, "1")
}

/// A statechart that holds the variables `data` and, on the event `on`, sends
/// `counted.sent` with `kept` given by `kept` and `hops` by `hops`.
fn counted_send(data: &str, on: &str, kept: &str, hops: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="counted_sender">
  <sce:import kind="event-schema" src="event_counted.scxml" as="Counted"/>
  <datamodel>
    {data}
  </datamodel>
  <state id="idle">
    <transition event="{on}" target="idle">
      <send event="counted.sent" target="#_internal">
        <param name="kept" expr="{kept}"/>
        <param name="hops" expr="{hops}"/>
      </send>
    </transition>
  </state>
</scxml>
"##
    )
}

/// Pinned by `Judge::wire_param` (`sce-build/src/forge/static_datamodel.rs`),
/// which puts a payload in scope for a scalar `<param>` where the transition's
/// event declares a schema at all: a scalar field of a schema that also holds
/// a list is a typed scalar like any other, and is sent on every backend that
/// sends a composed value, beside the list forwarded by its name. Where the
/// event declares no schema, a read of the payload is still refused.
#[test]
fn a_scalar_of_a_payload_that_holds_a_list_is_sent_as_any_scalar() {
    let forward = counted_send("", "counted.sent", "_event.data.kept", "_event.data.hops");
    for (language, row) in checked_backends() {
        if row.sends {
            let (ok, out) = run(&["check", "-l", language], &forward);
            assert!(
                ok,
                "{language} forwards a scalar of a payload that holds a list:\n{out}"
            );
        }
    }
    let unscoped = counted_send(
        r#"<data id="held" sce:type="list&lt;uint8&gt;[2]"/>"#,
        "go",
        "held",
        "_event.data.hops",
    );
    let (ok, out) = run(&["check", "-l", "rust"], &unscoped);
    assert!(
        !ok,
        "`go` declares no schema, so no payload is in scope:\n{out}"
    );
    assert!(
        out.contains("the triggering event's payload is read only in a transition"),
        "{out}"
    );
}

/// Pinned by `fits_but_for_count` (`sce-build/src/forge/value_type.rs`), which
/// `Judge::composed_param` asks (`sce-build/src/forge/static_datamodel.rs`):
/// a list variable, which states no lower count, is sent to a field that states
/// one, its count left to the send (docs/adr/0086, section 15.6), on each
/// backend that sends a composed value; one that may hold more than the field
/// is still refused where it is written.
#[test]
fn a_list_variable_is_sent_to_a_field_that_states_a_lower_count() {
    let fits = counted_sender(
        r#"<data id="held" sce:type="list&lt;uint8&gt;[2]"/>"#,
        "held",
    );
    for (language, row) in checked_backends() {
        if row.sends {
            let (ok, out) = run(&["check", "-l", language], &fits);
            assert!(
                ok,
                "{language} sends a list its count is held to when it runs:\n{out}"
            );
        }
    }
    let wide = counted_sender(
        r#"<data id="wide" sce:type="list&lt;uint8&gt;[3]"/>"#,
        "wide",
    );
    let (ok, out) = run(&["check", "-l", "rust"], &wide);
    assert!(!ok, "a list of 3 may not fit a field of 2:\n{out}");
    assert!(
        out.contains("'wide' is a list<uint8>[3], which may hold more than"),
        "{out}"
    );
}

/// Pinned by `composed_wire_value`'s count (`sce-build/src/forge/static_lowering.rs`)
/// and `RustComposed::send_counted` (`sce-build/src/forge/composed_lowering/rust.rs`):
/// the Rust machine of `static_composed_send` holds `held` to the one element
/// `counted.sent` asks before it writes it, through the `AlgorithmError` its
/// `<send>` reads a param's failure from, and holds no count of a list that
/// proves its own.
#[test]
fn a_list_sent_short_of_its_fields_count_is_held_to_it_on_rust() {
    let code = sender_machine();
    holds(
        &code,
        &[
            "::sce_forge_runtime::algorithm::holds_at_least(",
            "held.len(), 1)?;",
        ],
    );
    assert_eq!(
        code.matches("holds_at_least(").count(),
        2,
        "the two sends of `held` hold its count, and no other send does:\n{code}"
    );
}

/// Pinned by `Param::native_send_least` (`sce-build/src/model.rs`), which
/// `composed_wire_value` sets (`sce-build/src/forge/static_lowering.rs`), and
/// the `<send>` of `tools/codegen/templates/go/actions/send.go.jinja2`: the Go
/// machine of `static_composed_send` refuses `counted.sent` whole where `held`
/// is short of the one element its field asks, the one error raised and the
/// block ended before the event is delivered. Left out as a pair that could not
/// be read, the rest of the event was sent and refused again where it was
/// read, two errors for the one short list (docs/adr/0086, section 15.6).
#[test]
fn a_list_sent_short_of_its_fields_count_refuses_the_go_send_whole() {
    let code = generated_sender(
        sce_build::generator::Language::Go,
        "static_composed_send_sm.go",
    );
    let refusal = "<send> <param name='kept'> holds fewer elements than the 1 its field's type \
                   declares, so the event is not sent";
    assert_eq!(
        code.matches(refusal).count(),
        2,
        "the two sends of `held` refuse the event whole where it is short:\n{code}"
    );
    assert!(
        !code.contains("<send> <param name='kept'> failed to evaluate"),
        "a short list is not a pair left out of an event still sent:\n{code}"
    );
    for (at, _) in code.match_indices(refusal) {
        let rest = &code[at..];
        let delivered = rest
            .find("engine.Raise(meta)")
            .expect("the send delivers its event below the refusal");
        assert!(
            rest[..delivered]
                .lines()
                .any(|line| line.trim() == "return"),
            "the refusal ends the block before the event is delivered:\n{}",
            &rest[..delivered]
        );
    }
}

/// Pinned by the place `Judge::composed_param` asks the value for
/// (`sce-build/src/forge/static_datamodel.rs`).
#[test]
fn a_value_that_names_no_place_is_refused() {
    let document = sender(
        r#"<data id="wide" sce:type="list&lt;uint8&gt;[3]"/>"#,
        r#"<param name="late" expr="len(wide)"/>"#,
    );
    let (ok, out) = run(&["check", "-l", "rust"], &document);
    assert!(!ok, "a length is no list:\n{out}");
    assert!(out.contains("scxml/static-datamodel-rule"), "{out}");
    assert!(out.contains("names no such place"), "{out}");
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// a backend that does not send a composed value yet refuses the document by
/// name, never generating it. A whole send gives every composed field of the
/// first revision from a variable of its type, and such a backend refuses the
/// first of those variables before it reaches the `<send>`; the gate's `<send>`
/// arm is held by `a_send_or_raise_of_the_event_of_a_composed_schema_is_refused_by_name`.
#[test]
fn a_backend_that_sends_none_refuses_the_send_by_name() {
    let document = sender(
        r#"<data id="kept" sce:type="list&lt;uint8&gt;[2]"/>"#,
        r#"<param name="late" expr="kept"/>"#,
    );
    for (language, row) in checked_backends() {
        let (ok, out) = run(&["check", "-l", language], &document);
        if row.sends {
            assert!(ok, "{language} sends a list variable that fits:\n{out}");
        } else {
            assert!(!ok, "{language} sends no composed value yet:\n{out}");
            assert!(
                out.contains("generate/unsupported-feature"),
                "{language}: {out}"
            );
            assert!(
                out.contains(&format!("has no {language} lowering of its list")),
                "{language}: {out}"
            );
        }
    }
}
