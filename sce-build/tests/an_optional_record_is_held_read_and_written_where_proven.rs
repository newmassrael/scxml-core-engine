// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A record that may be absent, `optional<record:X>` (docs/adr/0055, step 9): a
//! `sce-static` statechart's variable starts at `null`, takes a whole record from a
//! payload, a record variable, a list's element or an algorithm's answer, is read as
//! `r.value.f` only where a test proves it present, and has a field written as
//! `r.value.f = e` only there. An algorithm holds one in a parameter, a return and a
//! local under the same rule. The Interpreter's lowering spells all of it (a record is
//! a plain object, absent is `null`); each generated backend refuses a document that
//! holds one by name, `generate/unsupported-feature` ("an optional record has no
//! <language> lowering yet"), until its own step.
//!
//! The scenario `static_optional_record.json` replays the fixture machine on the
//! engines that lower it; this file holds what the build answers before anything runs.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: a variable of `optional<record:X>` was
//! declared and refused by every backend, `.value` of one was refused for every
//! document, and no assignment, algorithm slot or field write could name one.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents of the fixture directory a probe imports, copied beside it: the
/// record `Item`, the algorithm that answers an optional one, a second record
/// (`Day`) and an enum.
const SIBLINGS: [&str; 5] = [
    "schema_item_pick.scxml",
    "algorithm_item_pick_first.scxml",
    "schema_day.scxml",
    "enum_item_status.scxml",
    "static_optional_record.scxml",
];

/// `sce-codegen <args…> <file>` in a directory holding the siblings and
/// `documents`, `file` being one of them or a sibling. Returns `(exit_ok, stderr +
/// stdout, the text of every file written)`.
fn run(args: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file));
    if matches!(args.first(), Some(&"generate") | Some(&"lower")) {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    let mut written = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            written.push('\n');
        }
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written,
    )
}

/// The `generate` arguments of each backend that emits a statechart's machine.
fn every_backend() -> Vec<Vec<&'static str>> {
    sce_build::forge::codegen_matrix::emitting(sce_build::forge::model::ForgeKind::Statechart)
        .map(|language| {
            let mut args = vec!["generate", "-l", language.canonical_name()];
            if language == sce_build::generator::Language::Go {
                args.extend(["--go-module-prefix", "example.test/generated"]);
            }
            args
        })
        .collect()
}

/// The generated backends, by the language `generate -l` names, whose step 9 of
/// docs/adr/0055 has landed: each lowers an optional record, and is held to its own
/// spellings by its own test; the rest refuse one by name until theirs. A backend
/// joins here in its step.
const LOWERING_AN_OPTIONAL_RECORD: &[&str] = &["python"];

/// [`every_backend`] without the ones that lower an optional record
/// ([`LOWERING_AN_OPTIONAL_RECORD`]): the ones that still refuse one.
fn backends_refusing_an_optional_record() -> Vec<Vec<&'static str>> {
    every_backend()
        .into_iter()
        .filter(|args| {
            !args
                .get(2)
                .is_some_and(|lang| LOWERING_AN_OPTIONAL_RECORD.contains(lang))
        })
        .collect()
}

/// A `sce-static` statechart that imports `Item` and `Day`, declares `data` beside
/// `pick`, an optional record of `Item`, a record `draft`, a list `items`, a number
/// `count`, a text `title`, a record `day` of the other schema and a list `days` of
/// it, and whose one state, `idle`, has `transitions`.
fn statechart(data: &str, transitions: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_record_probe">
  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="pick" sce:type="optional&lt;record:Item&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;record:Item&gt;" expr="null"/>
    <data id="draft" sce:type="record:Item">
      <sce:set name="id" expr="0"/>
      <sce:set name="title" expr="'draft'"/>
      <sce:set name="count" expr="0"/>
    </data>
    <data id="items" sce:type="list&lt;record:Item&gt;" sce:capacity="3"/>
    <data id="day" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>
      <sce:set name="dayOfMonth" expr="24"/>
    </data>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="count" sce:type="int32" expr="0"/>
    <data id="title" sce:type="string" sce:capacity="16" expr="''"/>
    <data id="same" sce:type="bool" expr="false"/>
    {data}
  </datamodel>
  <state id="idle">
    {transitions}
  </state>
</scxml>
"#
    )
}

/// [`statechart`] with `body` the content of its transition on `go`.
fn on_go(body: &str) -> String {
    statechart(
        "",
        &format!(r#"<transition event="go">{body}</transition>"#),
    )
}

/// `text` importing the algorithm `FirstAtLeast`, which a document that calls it
/// declares: an imported algorithm nothing calls is refused.
fn calling_first_at_least(text: &str) -> String {
    text.replace(
        "  <datamodel>",
        "  <sce:import kind=\"algorithm\" src=\"algorithm_item_pick_first.scxml\" as=\"FirstAtLeast\"/>\n  <datamodel>",
    )
}

/// `text` is refused by `check -l rust` with `code`, and the refusal holds `needle`.
fn refused(text: &str, code: &str, needle: &str) {
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", text)],
        "probe.scxml",
    );
    assert!(!ok, "expected a refusal with {code}:\n{out}");
    assert!(out.contains(code), "expected the code {code} in:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

/// `text` is judged fit and lowered for the Interpreter: `Ok` is the lowered text.
fn lowered(text: &str) -> String {
    let (ok, out, written) = run(&["lower"], &[("probe.scxml", text)], "probe.scxml");
    assert!(ok, "the Interpreter's lowering lowers the document:\n{out}");
    written
}

// ── The machine of the scenario ──────────────────────────────────────────

/// The fixture machine of `static_optional_record.json` is lowered for the
/// Interpreter: an optional record taken whole from a payload, a record variable, a
/// list's element and an algorithm's answer, read through `pick.value.<field>` and
/// written the same way, each as the plain object it is.
#[test]
fn the_interpreter_lowers_the_optional_record_scenario_machine() {
    let (ok, out, written) = run(&["lower"], &[], "static_optional_record.scxml");
    assert!(ok, "the ecmascript lowering lowers the machine:\n{out}");
    for needle in [
        // A field of the present record is the object's member.
        "pick.title",
        "pick.count",
        // A field written is the object, written again with that field changed.
        "SceStatic.set(pick, &apos;count&apos;,",
        "SceStatic.set(pick, &apos;title&apos;,",
        // A list's element, read whole by the library.
        "SceStatic.at(items, 1)",
        // The algorithm's answer, taken whole.
        "SceStatic.algorithms.item_pick_first(items, 5)",
    ] {
        assert!(
            written.contains(needle),
            "the lowered document holds `{needle}`:\n{written}"
        );
    }
}

/// Each generated backend refuses the machine by name, as a feature it does not
/// lower yet, until its own step of docs/adr/0055.
#[test]
fn every_generated_backend_refuses_the_optional_record_by_name() {
    for args in backends_refusing_an_optional_record() {
        let (ok, out, _) = run(&args, &[], "static_optional_record.scxml");
        assert!(!ok, "{args:?} must refuse the machine:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature")
                && out.contains("an optional record has no")
                && out.contains("lowering yet"),
            "{args:?} refuses it by name:\n{out}"
        );
    }
}

// ── Where it is read and written ─────────────────────────────────────────

/// A field of the present record is read where a test proves it present, under
/// every construct that proves one: an `<if>` with its `<elseif>` and `<else>`, the
/// right operand of `&&` and `||`, and the branches of a conditional.
#[test]
fn a_field_is_read_where_a_test_proves_the_record_present() {
    lowered(&on_go(
        r#"<if cond="pick !== null">
             <assign location="count" expr="pick.value.count"/>
           </if>
           <if cond="pick === null">
             <assign location="count" expr="0"/>
           <elseif cond="pick.value.count &gt; 1"/>
             <assign location="title" expr="pick.value.title"/>
           <else/>
             <assign location="title" expr="'low'"/>
           </if>
           <assign location="same" expr="pick !== null &amp;&amp; pick.value.count &gt; 1"/>
           <assign location="same" expr="pick === null || pick.value.count &gt; 1"/>
           <assign location="count" expr="pick !== null ? pick.value.count : 0"/>
           <assign location="count" expr="!(pick === null) ? pick.value.count : 0"/>"#,
    ));
}

/// A read nothing proves is `expression/optional-read-unproven`, placed at the read
/// and naming the record and the test that makes it present.
#[test]
fn a_field_is_not_read_where_nothing_proves_the_record_present() {
    let text = on_go(r#"<assign location="title" expr="pick.value.title"/>"#);
    refused(
        &text,
        "expression/optional-read-unproven",
        "pick.value.title",
    );
    let (_, out, _) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", &text)],
        "probe.scxml",
    );
    assert!(
        out.contains("`pick !== null`"),
        "naming the test that proves it:\n{out}"
    );
    // The test of the wrong sense proves nothing where it is true.
    refused(
        &on_go(
            r#"<if cond="pick === null"><assign location="count" expr="pick.value.count"/></if>"#,
        ),
        "expression/optional-read-unproven",
        "pick.value.count",
    );
    // A test in a transition's `cond` proves a variable nowhere in its content: the
    // exit actions and the other transitions of the microstep run between the two.
    refused(
        &statechart(
            "",
            r#"<transition event="go" cond="pick !== null">
                 <assign location="count" expr="pick.value.count"/>
               </transition>"#,
        ),
        "expression/optional-read-unproven",
        "pick.value.count",
    );
}

/// A field is written where a test proves the record present, and two fields one
/// after the other: a write of a field changes the record and not whether it is
/// present.
#[test]
fn a_field_is_written_where_a_test_proves_the_record_present() {
    let written = lowered(&on_go(
        r#"<if cond="pick !== null">
             <assign location="pick.value.count" expr="pick.value.count + 1"/>
             <assign location="pick.value.title" expr="'bumped'"/>
             <assign location="count" expr="1"/>
             <assign location="pick.value.count" expr="2"/>
           </if>"#,
    ));
    // Each write is the object `pick` holds, written again with the field changed.
    assert!(
        written.contains("SceStatic.set(pick, &apos;title&apos;,")
            && written.contains("SceStatic.set(pick, &apos;count&apos;,"),
        "each field is written through the library:\n{written}"
    );
}

/// A write nothing proves is refused at the write, naming the record and the
/// test; so is one after the statement that ends the proof.
#[test]
fn a_field_is_not_written_where_nothing_proves_the_record_present() {
    refused(
        &on_go(r#"<assign location="pick.value.count" expr="1"/>"#),
        "expression/unsupported-construct",
        "where no test proves `pick` present",
    );
    // An assignment to the record ends the proof: what it writes may be `null`.
    refused(
        &on_go(
            r#"<if cond="pick !== null">
                 <assign location="pick" expr="null"/>
                 <assign location="pick.value.count" expr="1"/>
               </if>"#,
        ),
        "expression/unsupported-construct",
        "where no test proves `pick` present",
    );
    // So does a write of a record taken whole into it, whatever it is.
    refused(
        &on_go(
            r#"<if cond="pick !== null">
                 <assign location="pick" expr="draft"/>
                 <assign location="count" expr="pick.value.count"/>
               </if>"#,
        ),
        "expression/optional-read-unproven",
        "pick.value.count",
    );
}

/// A field the schema does not declare is no member of the present record.
#[test]
fn the_present_record_has_only_the_fields_of_its_schema() {
    refused(
        &on_go(
            r#"<if cond="pick !== null"><assign location="count" expr="pick.value.nope"/></if>"#,
        ),
        "expression/unknown-member",
        "nope",
    );
}

/// The present record is read through its fields: taken whole as `pick.value` it is
/// no expression (a record is no operand), and no default can be given for an
/// absent one.
#[test]
fn the_present_record_is_no_expression_and_an_absent_one_has_no_default() {
    // Nothing in an expression makes a record, so a whole record is taken by name,
    // and `pick.value` is no name.
    refused(
        &on_go(r#"<if cond="pick !== null"><assign location="draft" expr="pick.value"/></if>"#),
        "scxml/static-datamodel-rule",
        "is taken by name",
    );
    refused(
        &on_go(r#"<if cond="pick !== null"><assign location="count" expr="pick.value"/></if>"#),
        "expression/unsupported-construct",
        "taken whole",
    );
    refused(
        &on_go(r#"<assign location="count" expr="pick ?? 0"/>"#),
        "expression/unsupported-construct",
        "with a default",
    );
    // An assignment to `pick.value` is no assignment: a record is put into `pick`.
    refused(
        &on_go(r#"<assign location="pick.value" expr="draft"/>"#),
        "expression/unsupported-construct",
        "assigned whole",
    );
}

/// Two optional records are not compared: a record is no operand. A test against
/// `null` is the one comparison of a whole record.
#[test]
fn an_optional_record_is_tested_against_null_and_compared_with_nothing_else() {
    refused(
        &on_go(r#"<assign location="same" expr="pick === other"/>"#),
        "expression/unsupported-construct",
        "other than `null`",
    );
    lowered(&on_go(
        r#"<assign location="same" expr="pick === null"/>
           <assign location="same" expr="null !== other"/>"#,
    ));
}

// ── What is put into it ──────────────────────────────────────────────────

/// `null` makes it absent, and a whole record is put into it by the rule a record
/// variable takes one by: a record variable, a payload, a list's element, what an
/// imported algorithm answers, and another optional record.
#[test]
fn a_whole_record_is_put_into_it_as_a_record_variable_takes_one() {
    let text = statechart(
        "",
        r#"<transition event="item.picked">
             <assign location="pick" expr="_event.data"/>
             <assign location="other" expr="pick"/>
           </transition>
           <transition event="go">
             <assign location="pick" expr="draft"/>
             <assign location="pick" expr="items[0]"/>
             <assign location="pick" expr="FirstAtLeast(items, 5)"/>
             <assign location="pick" expr="null"/>
           </transition>"#,
    );
    lowered(&calling_first_at_least(&text));
}

/// A record of another schema is no value of it, and a value that is no record's
/// name is refused, saying which names are taken.
#[test]
fn only_a_record_of_its_schema_is_put_into_it() {
    refused(
        &on_go(r#"<assign location="pick" expr="day"/>"#),
        "scxml/static-datamodel-rule",
        "a whole record of the schema Item is taken by name",
    );
    refused(
        &on_go(r#"<assign location="pick" expr="days[0]"/>"#),
        "scxml/static-datamodel-rule",
        "is a record of the schema Day",
    );
    refused(
        &on_go(r#"<assign location="pick" expr="3"/>"#),
        "scxml/static-datamodel-rule",
        "taken by name",
    );
}

/// It starts absent: a record is built whole from `<sce:set>` children, which a
/// variable that may hold none has no use for.
#[test]
fn an_optional_record_starts_absent() {
    refused(
        &statechart(
            r#"<data id="late" sce:type="optional&lt;record:Item&gt;" expr="draft"/>"#,
            "",
        ),
        "scxml/static-datamodel-rule",
        "starts absent",
    );
}

/// An optional record is no optional of an optional, and no field of an event
/// schema: the nested form is refused on the `sce:type`, naming what may stand
/// inside.
#[test]
fn an_optional_of_an_optional_record_is_refused() {
    refused(
        &statechart(
            r#"<data id="late" sce:type="optional&lt;optional&lt;record:Item&gt;&gt;" expr="null"/>"#,
            "",
        ),
        "validation/invalid-attribute",
        "optional<record:Item>",
    );
}

// ── An algorithm holds one ───────────────────────────────────────────────

/// An algorithm that takes an optional record `x` and a list of records, keeps one
/// of them in a local `held` that starts absent, and answers the count of `x` where
/// it is present, or of the local, or -1. `body` is what it runs.
fn algorithm(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="pick_count" version="1.0">
  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>
  <sce:signature>
    <sce:param name="x" type="optional&lt;record:Item&gt;"/>
    <sce:param name="items" type="list&lt;record:Item&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="optional&lt;record:Item&gt;" init="null"/>
    {body}
  </sce:body>
</scxml>
"#
    )
}

/// The algorithm of [`algorithm`] is called from a statechart with a variable that
/// may be absent, with `null` and with a record, which is lifted: each argument is
/// held by name, and the Interpreter's lowering hands the function the value the
/// variable holds.
#[test]
fn an_algorithm_takes_an_optional_record_by_name_and_reads_it_where_proven() {
    let body = r#"<sce:foreach item="it" in="items">
      <sce:if cond="held === null">
        <sce:assign target="held" expr="it"/>
      </sce:if>
    </sce:foreach>
    <sce:if cond="x !== null">
      <sce:return expr="x.value.count"/>
    </sce:if>
    <sce:if cond="held !== null">
      <sce:assign target="held.value.count" expr="7"/>
      <sce:return expr="held.value.count"/>
    </sce:if>
    <sce:return expr="-1"/>"#;
    let caller = statechart(
        r#"<data id="seen" sce:type="int32" expr="0"/>"#,
        r#"<transition event="go">
             <assign location="seen" expr="PickCount(pick, items)"/>
             <assign location="seen" expr="PickCount(null, items)"/>
             <assign location="seen" expr="PickCount(draft, items)"/>
           </transition>"#,
    )
    .replace(
        "  <datamodel>",
        "  <sce:import kind=\"algorithm\" src=\"algorithm_pick_count.scxml\" as=\"PickCount\"/>\n  <datamodel>",
    );
    let (ok, out, written) = run(
        &["lower"],
        &[
            ("algorithm_pick_count.scxml", &algorithm(body)),
            ("probe.scxml", &caller),
        ],
        "probe.scxml",
    );
    assert!(ok, "the ecmascript lowering lowers the machine:\n{out}");
    // `x.value.count` is the member of the object `x` is, and the field written is
    // the object, written again with that field changed.
    for needle in [
        "x.count",
        "held = SceStatic.set(held, &apos;count&apos;, 7);",
    ] {
        assert!(
            written.contains(needle),
            "the lowered document holds `{needle}`:\n{written}"
        );
    }
}

/// The body of an algorithm is held to the same rule: a read nothing proves is
/// `expression/optional-read-unproven`, and a write of a field where nothing does is
/// refused at the write.
#[test]
fn an_algorithms_optional_record_is_read_and_written_only_where_proven() {
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[(
            "algorithm_pick_count.scxml",
            &algorithm(r#"<sce:return expr="x.value.count"/>"#),
        )],
        "algorithm_pick_count.scxml",
    );
    assert!(!ok, "an unproven read is refused:\n{out}");
    assert!(
        out.contains("expression/optional-read-unproven") && out.contains("x.value.count"),
        "refused at the read:\n{out}"
    );
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[(
            "algorithm_pick_count.scxml",
            &algorithm(
                r#"<sce:assign target="held.value.count" expr="7"/>
    <sce:return expr="-1"/>"#,
            ),
        )],
        "algorithm_pick_count.scxml",
    );
    assert!(!ok, "an unproven write is refused:\n{out}");
    assert!(
        out.contains("where no test proves `held` present"),
        "refused at the write:\n{out}"
    );
}

/// A record put into an optional record is of its schema: one of another is refused
/// at the statement that puts it.
#[test]
fn an_algorithm_puts_into_an_optional_record_a_record_of_its_schema() {
    let text = algorithm(
        r#"<sce:foreach item="it" in="items">
      <sce:assign target="held" expr="it"/>
    </sce:foreach>
    <sce:return expr="-1"/>"#,
    )
    .replace(
        "  <sce:signature>",
        "  <sce:import kind=\"event-schema\" src=\"schema_day.scxml\" as=\"Day\"/>\n  <sce:signature>",
    )
    .replace("list&lt;record:Item&gt;", "list&lt;record:Day&gt;");
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[("algorithm_pick_count.scxml", &text)],
        "algorithm_pick_count.scxml",
    );
    assert!(!ok, "a record of another schema is refused:\n{out}");
    assert!(
        out.contains("a record of `Day`") && out.contains("optional record of `Item`"),
        "refused naming both schemas:\n{out}"
    );
}

/// An algorithm that holds an optional record is refused by every generated
/// backend, by name, where it renders the slot: its own step spells one.
#[test]
fn every_generated_backend_refuses_an_algorithm_that_holds_an_optional_record_by_name() {
    let text = algorithm(r#"<sce:return expr="-1"/>"#);
    for args in backends_refusing_an_optional_record() {
        let (ok, out, _) = run(
            &args,
            &[("algorithm_pick_count.scxml", &text)],
            "algorithm_pick_count.scxml",
        );
        assert!(!ok, "{args:?} must refuse the algorithm:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature")
                && out.contains("an optional value has no")
                && out.contains("lowering yet"),
            "{args:?} refuses it by name:\n{out}"
        );
    }
}
