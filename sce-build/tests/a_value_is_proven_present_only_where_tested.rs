// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The value of an optional value is read only where a test proves it present
//! (docs/adr/0055, section 3 and step 3). Each construct that proves a fact is
//! held by a read it admits and a read it does not, and each statement that
//! ends a fact by the read it refuses after it; a read the build has not proven
//! is `expression/optional-read-unproven`, placed at the read. One analysis
//! (`forge::narrowing`) serves a statechart's executable content and an
//! algorithm's body, so both are held here.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-21: nothing read the value of an optional
//! value, and nothing said where one is present.

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

/// The enum the schema imports (`Status`), copied beside each probe.
const ENUM_FILE: &str = "enum_item_status.scxml";

/// The file name a probe imports the schema [`SCHEMA_DUE`] from.
const SCHEMA_DUE_FILE: &str = "schema_item_due.scxml";

/// An event schema whose fields may be absent beside one every delivery
/// carries.
const SCHEMA_DUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_item_due" sce:event-name="item.due">
  <sce:import src="enum_item_status.scxml" kind="enum" as="Status"/>
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:max-size="16" sce:direction="in"/>
    <data id="status" sce:type="optional&lt;enum:Status&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A record of [`SCHEMA_DUE`] declared as the variable `id`, every optional
/// field absent.
fn record_variable(id: &str) -> String {
    format!(
        r#"<data id="{id}" sce:type="record:Due">
      <sce:set name="id" expr="1"/>
      <sce:set name="due" expr="null"/>
      <sce:set name="title" expr="null"/>
      <sce:set name="status" expr="null"/>
    </data>"#
    )
}

/// A `sce-static` statechart that imports `Status` and [`SCHEMA_DUE`] as `Due`
/// and declares an optional day `due`, a number `count`, an optional text
/// `sid` an id fits, two records `rec` and `other` and a list of records
/// `items`; its one state, `idle`, has a transition on `tick` whose content is
/// `content`.
fn on_tick(content: &str) -> String {
    statechart(&format!(
        r#"<transition event="tick">{content}</transition>"#
    ))
}

/// [`on_tick`]'s statechart with `body` the content of `idle`.
fn statechart(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="narrowing_probe">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="{SCHEMA_DUE_FILE}" as="Due"/>
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="count" sce:type="int64" expr="0"/>
    <data id="sid" sce:type="optional&lt;string&gt;" sce:capacity="40" expr="null"/>
    {}
    {}
    <data id="items" sce:type="list&lt;record:Due&gt;" sce:capacity="4"/>
  </datamodel>
  <state id="idle">{body}</state>
</scxml>
"#,
        record_variable("rec"),
        record_variable("other"),
    )
}

/// An algorithm that takes a record of [`SCHEMA_DUE`] as `item` and a list of
/// them as `items`, returns an `int64` (declaring `may-fail` when `may_fail`)
/// and runs `body`.
fn algorithm(may_fail: bool, body: &str) -> String {
    let fails = if may_fail { r#" may-fail="true""# } else { "" };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="due_probe" version="1.0">
  <sce:import kind="event-schema" src="{SCHEMA_DUE_FILE}" as="Due"/>
  <sce:signature>
    <sce:param name="item" type="record:Due"/>
    <sce:param name="items" type="list&lt;record:Due&gt;"/>
    <sce:return type="int64"{fails}/>
  </sce:signature>
  <sce:body>
    <sce:var name="last" type="int64" init="0"/>
    <sce:var name="going" type="bool" init="true"/>
    {body}
  </sce:body>
</scxml>
"#
    )
}

/// `sce-codegen <args…> <file>` in a directory holding the enum, the schema and
/// `text` as `file`. Returns `(exit_ok, stderr + stdout)`.
fn run(args: &[&str], text: &str, file: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(fixtures().join(ENUM_FILE), dir.path().join(ENUM_FILE)).expect("copy the enum");
    std::fs::write(dir.path().join(SCHEMA_DUE_FILE), SCHEMA_DUE).expect("write the schema");
    std::fs::write(dir.path().join(file), text).expect("write the probe");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file));
    if matches!(args.first(), Some(&"generate")) {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// The refusal a backend gives a document that holds an optional value, before
/// the step that lowers one there.
const NOT_LOWERED: &str = "an optional value has no";

/// The code of a read the build has not proven present.
const UNPROVEN: &str = "expression/optional-read-unproven";

/// `text`, written as `file`, is judged fit, and no read in it is unproven: a
/// statechart is generated for Rust, whose step of docs/adr/0055 lowers it, and
/// an algorithm is refused only where the backend renders it, by name, until the
/// step that lowers an algorithm's optional value (8).
fn proven(text: &str, file: &str) {
    let (ok, out) = run(&["generate", "-l", "rust"], text, file);
    assert!(
        !out.contains("\"code\":\"expression/") && !out.contains("scxml/static-datamodel-rule"),
        "every read is proven:\n{out}"
    );
    if file == ALGORITHM {
        assert!(
            !ok,
            "no backend renders an algorithm's optional value yet:\n{out}"
        );
        assert!(
            out.contains(NOT_LOWERED),
            "refused only where it is rendered:\n{out}"
        );
    } else {
        assert!(ok, "Rust lowers the statechart:\n{out}");
    }
}

/// `text`, written as `file`, is refused at a read of `read` the build has not
/// proven present, naming the optional value `optional` and both repairs.
fn unproven(text: &str, file: &str, read: &str, optional: &str) {
    let (ok, out) = run(&["generate", "-l", "rust"], text, file);
    assert!(!ok, "expected {UNPROVEN}:\n{out}");
    assert!(out.contains(UNPROVEN), "expected {UNPROVEN} in:\n{out}");
    assert!(
        out.contains(&format!("\"actual\":\"{read}\"")),
        "placed at the read `{read}`:\n{out}"
    );
    assert!(
        out.contains(&format!("`{optional} !== null`"))
            && out.contains(&format!("`{optional} ?? ")),
        "naming the test and the default for `{optional}`:\n{out}"
    );
}

/// A statechart probe's file name.
const PROBE: &str = "probe.scxml";

/// An algorithm probe's file name.
const ALGORITHM: &str = "due_probe.scxml";

// ── Within one expression ────────────────────────────────────────────────

#[test]
fn the_right_operand_of_and_is_read_where_the_left_is_true() {
    proven(
        &on_tick(
            r#"<if cond="due !== null &amp;&amp; due.value &gt; 3"><assign location="count" expr="1"/></if>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="due === null &amp;&amp; due.value &gt; 3"><assign location="count" expr="1"/></if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

#[test]
fn the_right_operand_of_or_is_read_where_the_left_is_false() {
    proven(
        &on_tick(
            r#"<if cond="due === null || due.value &gt; 3"><assign location="count" expr="1"/></if>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="due !== null || due.value &gt; 3"><assign location="count" expr="1"/></if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

#[test]
fn the_branches_of_a_conditional_are_read_where_its_condition_is_true_and_false() {
    proven(
        &on_tick(
            r#"<assign location="count" expr="due !== null ? due.value : 0"/>
               <assign location="count" expr="due === null ? 0 : due.value"/>
               <assign location="count" expr="!(due === null) ? due.value : 0"/>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(r#"<assign location="count" expr="due === null ? due.value : 0"/>"#),
        PROBE,
        "due.value",
        "due",
    );
}

/// Nothing proves a read with no test before it.
#[test]
fn a_read_with_no_test_is_unproven() {
    unproven(
        &on_tick(r#"<assign location="count" expr="due.value"/>"#),
        PROBE,
        "due.value",
        "due",
    );
}

// ── An <if> chain ────────────────────────────────────────────────────────

#[test]
fn an_if_block_is_read_where_its_condition_is_true_and_nothing_after_it() {
    proven(
        &on_tick(r#"<if cond="due !== null"><assign location="count" expr="due.value"/></if>"#),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="due !== null"><assign location="count" expr="1"/></if>
               <assign location="count" expr="due.value"/>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

#[test]
fn an_elseif_is_read_where_every_earlier_condition_is_false() {
    proven(
        &on_tick(
            r#"<if cond="due === null"><assign location="count" expr="1"/>
               <elseif cond="due.value &gt; 3"/><assign location="count" expr="due.value"/>
               </if>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="due !== null"><assign location="count" expr="1"/>
               <elseif cond="count &gt; 3"/><assign location="count" expr="due.value"/>
               </if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

#[test]
fn an_else_is_read_where_every_condition_is_false() {
    proven(
        &on_tick(
            r#"<if cond="due === null"><assign location="count" expr="1"/>
               <else/><assign location="count" expr="due.value"/>
               </if>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="due !== null"><assign location="count" expr="1"/>
               <else/><assign location="count" expr="due.value"/>
               </if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

// ── A transition's cond ──────────────────────────────────────────────────

/// A test of a payload field in the `cond` proves it in the content; a test of
/// a variable does not, since the microstep runs other content between them.
#[test]
fn a_transitions_cond_proves_a_payload_field_and_no_variable() {
    proven(
        &statechart(
            r#"<transition event="item.due" cond="_event.data.due !== null">
                 <assign location="count" expr="_event.data.due.value"/>
               </transition>"#,
        ),
        PROBE,
    );
    unproven(
        &statechart(
            r#"<transition event="tick" cond="due !== null">
                 <assign location="count" expr="due.value"/>
               </transition>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

// ── A loop ───────────────────────────────────────────────────────────────

/// A fact proven before a `<foreach>` holds in its body only if nothing in the
/// body ends it: the second turn follows the end of the first, so a write
/// after the read ends it too.
#[test]
fn a_foreach_keeps_a_fact_only_if_its_body_ends_none() {
    proven(
        &on_tick(
            r#"<if cond="due !== null">
                 <foreach array="items" item="it"><assign location="count" expr="due.value"/></foreach>
               </if>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="due !== null">
                 <foreach array="items" item="it">
                   <assign location="count" expr="due.value"/>
                   <assign location="due" expr="null"/>
                 </foreach>
               </if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

/// A field of the loop's item is a subject; an element read by its index is
/// not, since the index may change between the test and the read.
#[test]
fn a_loop_items_field_is_a_subject_and_an_indexed_element_is_not() {
    proven(
        &on_tick(
            r#"<foreach array="items" item="it">
                 <if cond="it.due !== null"><assign location="count" expr="it.due.value"/></if>
               </foreach>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="items[0].due !== null"><assign location="count" expr="items[0].due.value"/></if>"#,
        ),
        PROBE,
        "items[0].due.value",
        "items[0].due",
    );
}

// ── What ends a fact ─────────────────────────────────────────────────────

/// An assignment to the subject ends the fact, whatever it writes: only a
/// test proves presence.
#[test]
fn an_assignment_to_the_subject_ends_its_fact() {
    unproven(
        &on_tick(
            r#"<if cond="due !== null">
                 <assign location="due" expr="5"/>
                 <assign location="count" expr="due.value"/>
               </if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

/// An assignment to what holds the subject ends it; one to a field beside it
/// does not.
#[test]
fn an_assignment_above_the_subject_ends_its_fact_and_one_beside_it_does_not() {
    proven(
        &on_tick(
            r#"<if cond="rec.due !== null">
                 <assign location="rec.id" expr="2"/>
                 <assign location="count" expr="rec.due.value"/>
               </if>"#,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r#"<if cond="rec.due !== null">
                 <assign location="rec" expr="other"/>
                 <assign location="count" expr="rec.due.value"/>
               </if>"#,
        ),
        PROBE,
        "rec.due.value",
        "rec.due",
    );
}

/// A native `<script>` may write anything, and ends every fact.
#[test]
fn a_script_ends_every_fact() {
    unproven(
        &on_tick(
            r#"<if cond="due !== null">
                 <script><cpp>tick();</cpp></script>
                 <assign location="count" expr="due.value"/>
               </if>"#,
        ),
        PROBE,
        "due.value",
        "due",
    );
}

/// A `<send idlocation>` writes the id it generates to the variable it names.
#[test]
fn a_send_idlocation_ends_the_fact_about_its_variable() {
    proven(
        &on_tick(
            r##"<if cond="sid !== null">
                 <send event="ping" target="#_internal"/>
                 <log expr="sid.value"/>
               </if>"##,
        ),
        PROBE,
    );
    unproven(
        &on_tick(
            r##"<if cond="sid !== null">
                 <send event="ping" target="#_internal" idlocation="sid"/>
                 <log expr="sid.value"/>
               </if>"##,
        ),
        PROBE,
        "sid.value",
        "sid",
    );
}

// ── An algorithm's body ──────────────────────────────────────────────────

#[test]
fn an_algorithms_if_block_is_read_where_its_condition_is_true() {
    proven(
        &algorithm(
            false,
            r#"<sce:if cond="item.due !== null"><sce:assign target="last" expr="item.due.value"/></sce:if>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
    );
    unproven(
        &algorithm(
            false,
            r#"<sce:if cond="item.due === null"><sce:assign target="last" expr="item.due.value"/></sce:if>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
        "item.due.value",
        "item.due",
    );
}

#[test]
fn an_algorithms_else_is_read_where_its_condition_is_false() {
    proven(
        &algorithm(
            false,
            r#"<sce:if cond="item.due === null"><sce:assign target="last" expr="0"/>
      <sce:else><sce:assign target="last" expr="item.due.value"/></sce:else>
    </sce:if>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
    );
    unproven(
        &algorithm(
            false,
            r#"<sce:if cond="item.due !== null"><sce:assign target="last" expr="0"/>
      <sce:else><sce:assign target="last" expr="item.due.value"/></sce:else>
    </sce:if>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
        "item.due.value",
        "item.due",
    );
}

/// After an `<sce:if>` with no `<sce:else>` whose block returns on every path,
/// what follows is read where its condition is false.
#[test]
fn an_early_return_proves_what_follows_it() {
    proven(
        &algorithm(
            false,
            r#"<sce:if cond="item.due === null"><sce:return expr="0"/></sce:if>
    <sce:return expr="item.due.value"/>"#,
        ),
        ALGORITHM,
    );
    unproven(
        &algorithm(
            false,
            r#"<sce:if cond="item.due === null"><sce:assign target="last" expr="0"/></sce:if>
    <sce:return expr="item.due.value"/>"#,
        ),
        ALGORITHM,
        "item.due.value",
        "item.due",
    );
}

/// After `<sce:require>`, what follows is read where its condition is true.
#[test]
fn a_require_proves_what_follows_it() {
    proven(
        &algorithm(
            true,
            r#"<sce:require cond="item.due !== null"/>
    <sce:return expr="item.due.value"/>"#,
        ),
        ALGORITHM,
    );
    unproven(
        &algorithm(
            true,
            r#"<sce:require cond="item.id &gt; 0"/>
    <sce:return expr="item.due.value"/>"#,
        ),
        ALGORITHM,
        "item.due.value",
        "item.due",
    );
}

/// An `<sce:while>`'s condition proves its facts at the start of each turn.
#[test]
fn a_whiles_condition_proves_its_body() {
    proven(
        &algorithm(
            false,
            r#"<sce:while cond="going &amp;&amp; item.due !== null" max-iter="4">
      <sce:assign target="last" expr="item.due.value"/>
      <sce:assign target="going" expr="false"/>
    </sce:while>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
    );
    unproven(
        &algorithm(
            false,
            r#"<sce:while cond="going" max-iter="4">
      <sce:assign target="last" expr="item.due.value"/>
      <sce:assign target="going" expr="false"/>
    </sce:while>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
        "item.due.value",
        "item.due",
    );
}

/// A fact proven before an algorithm's loop holds in its body only if nothing
/// in the body ends it: the local record's field written in the body ends it.
#[test]
fn an_algorithms_loop_keeps_a_fact_only_if_its_body_ends_none() {
    let body = |write: &str| {
        format!(
            r#"<sce:var name="r" type="record:Due">
      <sce:set name="id" expr="1"/>
      <sce:set name="due" expr="item.due"/>
      <sce:set name="title" expr="null"/>
      <sce:set name="status" expr="null"/>
    </sce:var>
    <sce:if cond="r.due !== null">
      <sce:while cond="going" max-iter="4">
        <sce:assign target="last" expr="r.due.value"/>
        {write}
        <sce:assign target="going" expr="false"/>
      </sce:while>
    </sce:if>
    <sce:return expr="last"/>"#
        )
    };
    proven(&algorithm(false, &body("")), ALGORITHM);
    unproven(
        &algorithm(false, &body(r#"<sce:assign target="r.due" expr="null"/>"#)),
        ALGORITHM,
        "r.due.value",
        "r.due",
    );
}

/// A field of an `<sce:foreach>`'s record item is a subject in the body.
#[test]
fn an_algorithms_foreach_item_field_is_a_subject() {
    proven(
        &algorithm(
            false,
            r#"<sce:foreach item="e" in="items">
      <sce:if cond="e.due !== null"><sce:assign target="last" expr="e.due.value"/></sce:if>
    </sce:foreach>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
    );
    unproven(
        &algorithm(
            false,
            r#"<sce:foreach item="e" in="items">
      <sce:assign target="last" expr="e.due.value"/>
    </sce:foreach>
    <sce:return expr="last"/>"#,
        ),
        ALGORITHM,
        "e.due.value",
        "e.due",
    );
}
