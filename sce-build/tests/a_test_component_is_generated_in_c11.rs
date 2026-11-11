// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in C11 (docs/SCE_TEST_PROFILE.md §11.9, §12.3,
//! §12.6 slice 6): the statechart machine of the document, each branch a
//! transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile, a `<log>` and a send as calls on the component's runtime context,
//! a PTC through the host invoker door; and beside the machine the component
//! the profile's runtime drives, which constructs the machine with its doors,
//! sets its context on it and only then enters its initial configuration. The
//! twin of `a_test_component_is_generated_in_python.rs`, over the same
//! fixtures.
//!
//! What is C's own: a receive branch with a template matches through a
//! function of the machine's own, which binds the template's arguments to its
//! `matches` (C has no closure); a message's value is known by the name of its
//! record's C type (`<unit>_value_type`); a send lends its value from a local
//! of the block its request stands in; a text the content reads is copied into
//! the buffer the machine keeps for it; nothing is allocated, the component
//! being storage its factory owns. C11 holds a number, a `bool`, a text, a
//! byte string and an enum of a received message as the record's unit does, a
//! byte string's bytes and length copied into the array of the machine's
//! channel. A module
//! parameter starts at its type's zero, or its enum's first variant, and is
//! read from the run as a saved value of its type is read back, between the
//! construction of the machine and the entry into its initial configuration.
//! What C11 refuses is held here too: a component that declares
//! `<sce:action>`, and one whose template reaches a union, which C11's test
//! profile does not match yet.
//!
//! What the generated components do when they run is held by
//! `backends/c/tests/integration/test_a_test_component_runs_through_its_doors.c`,
//! which compiles them from the same fixtures and runs them through
//! `sce_test_case_run_t`.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::union_check::NOT_LOWERED_YET;
use sce_build::generator::Language;

/// The directory of the components the runtime's end-to-end test runs.
fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime")
}

/// The directory of the templates over imported records.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// The directory of the test profile's own fixtures.
fn profile_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile")
}

/// Every component of the runtime's end-to-end test that C11 generates, each
/// with whether it sends on a port and whether it creates a PTC.
const COMPONENTS: [(&str, bool, bool); 5] = [
    ("asker", true, false),
    ("coordinator", true, true),
    ("answerer", true, false),
    ("ptc_keeper", false, true),
    ("idler", false, false),
];

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::C11),
        Language::C11,
    )
    .map(|output| output.files)
}

/// The C11 files of the document at `path`, which generates.
fn files_at(path: &Path) -> Vec<(String, String)> {
    generate(path).unwrap_or_else(|e| panic!("{} generates on C11: {e:?}", path.display()))
}

/// The file `file` of the component `name`.
fn file(name: &str, file: &str) -> String {
    files_at(&runtime_dir().join(format!("{name}.scxml")))
        .into_iter()
        .find(|(written, _)| written == file)
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("{name} writes {file}"))
}

/// The whole C11 text of the document at `path`, which generates.
fn text_at(path: &Path) -> String {
    files_at(path)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole C11 text of the component `name`.
fn text(name: &str) -> String {
    text_at(&runtime_dir().join(format!("{name}.scxml")))
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`:\n{haystack}"))
}

/// The include of the generated header of the document `snake`.
fn included(snake: &str) -> String {
    format!("\n#include \"{snake}.h\"\n")
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on C11: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on C11: {other:?}"),
        },
        other => panic!("{name} on C11: {other:?}"),
    }
}

/// The detail of the refusal by name of the document at `path`.
fn refused(path: &Path, name: &str) -> String {
    let Err(err) = generate(path) else {
        panic!("{} generated on C11", path.display());
    };
    unsupported(name, err)
}

/// §12.3: each component is its machine's header and source, and beside the
/// machine the component the runtime drives: a struct whose first member is
/// what the run drives, made from storage the factory owns. Its machine keeps
/// the runtime context by value; the component gives the construction the
/// doors it registered, sets the context on the machine the construction has
/// cleared, and only then enters the initial configuration, so the first
/// `<onentry>` already reaches the run. A component registers only the doors
/// its machine goes through.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for (name, sends, creates) in COMPONENTS {
        let names: Vec<String> = files_at(&runtime_dir().join(format!("{name}.scxml")))
            .into_iter()
            .map(|(written, _)| written)
            .collect();
        assert_eq!(names, [format!("{name}_sm.h"), format!("{name}_sm.c")]);
        let header = file(name, &format!("{name}_sm.h"));
        for needle in [
            "\n#include \"sce/test/test_case_run.h\"\n".to_string(),
            "\n    sce_test_component_context_t test_context;\n".to_string(),
            format!("\ntypedef struct {name}_component {{\n"),
            "\n    sce_test_component_machine_t base;\n".to_string(),
            format!("\n    {name}_t machine;\n"),
            "\n    bool held;\n".to_string(),
            format!("\n}} {name}_component_t;\n"),
            format!(
                "\nsce_test_component_machine_t *{name}_component_make({name}_component_t \
                 *component);\n"
            ),
        ] {
            assert!(
                header.contains(&needle),
                "{name}'s header holds `{needle}`:\n{header}"
            );
        }
        let source = file(name, &format!("{name}_sm.c"));
        for needle in [
            format!(
                "SCE_SM_FN static void {name}_component_start(sce_test_component_machine_t *self,"
            ),
            format!("    {name}_t *sm = &(({name}_component_t *)(void *)self)->machine;\n"),
            "    sm->test_context = *context;\n".to_string(),
            format!("    {name}_init_enter(sm);\n"),
            format!(
                "    snapshot.event = {}_EVENT_SCE_TEST_SNAPSHOT;\n",
                name.to_uppercase()
            ),
            format!("    {name}_raise_external(sm, &snapshot);\n    {name}_step(sm);\n"),
            format!(
                "    return {name}_is_in_final_state(&((const {name}_component_t *)(const void \
                 *)self)->machine);\n"
            ),
            format!("    {name}_destroy(&component->machine);\n    component->held = false;\n"),
            format!("    .start = {name}_component_start,\n"),
            format!("    .release = {name}_component_release,\n"),
            format!(
                "SCE_SM_FN sce_test_component_machine_t *{name}_component_make({name}_component_t \
                 *component) {{\n    if (component == NULL || component->held) {{\n        \
                 return NULL;\n    }}\n"
            ),
        ] {
            assert!(
                source.contains(&needle),
                "{name}'s source holds `{needle}`:\n{source}"
            );
        }
        let start = &source[at(&source, &format!("{name}_component_start("))..];
        let constructed = at(
            start,
            &format!("{name}_init_construct(sm, false, sce_test_context_name(context)"),
        );
        let set = at(start, "sm->test_context = *context;");
        let entered = at(start, &format!("{name}_init_enter(sm);"));
        assert!(
            constructed < set && set < entered,
            "{name}: the context is set after the construction clears the machine and before \
             its initial configuration is entered"
        );
        let serves = format!(
            "    (void)sce_host_registry_register(&processors, \"http://sce.dev/ext/test-port\", \
             {name}_component_send, sm);\n"
        );
        let runs = format!(
            "    (void)sce_host_invoker_register(&invokers, \
             \"http://sce.dev/ext/test-component\", {name}_component_invoke, sm);\n"
        );
        assert_eq!(source.contains(&serves), sends, "{name}: {serves}");
        assert_eq!(source.contains(&runs), creates, "{name}: {runs}");
        if sends {
            assert!(
                at(start, &serves) < constructed,
                "{name}: the test-port processor is given to the construction"
            );
            // The request lends one `sce_test_message_value_t`, the value with
            // the record type the test log names and writes it by.
            assert!(source.contains(
                "    sce_test_context_send(&sm->test_context, request->target, \
                 request->event_name,\n                          \
                 sce_test_message_value_carried(request->value));\n"
            ));
        }
        if creates {
            assert!(
                at(start, &runs) < constructed,
                "{name}: the test-component invoker is given to the construction"
            );
            for needle in [
                "        sce_test_context_create(&sm->test_context, event->invoke_id);\n",
                "        sce_test_context_kill(&sm->test_context, event->invoke_id);\n",
            ] {
                assert!(source.contains(needle), "{name} holds `{needle}`");
            }
        }
    }
}

/// The construction of a machine and the entry into its initial configuration
/// are one call for a host (`_init`) and two for the component, which sets the
/// context between them; both halves are the ones `_init` runs, so a machine
/// started by its host enters as it did.
#[test]
fn the_entry_into_the_initial_configuration_is_its_own_half_of_init() {
    let source = file("idler", "idler_sm.c");
    let half = &source[at(
        &source,
        "SCE_SM_FN static void idler_init_enter(idler_t *sm) {\n",
    )..];
    let entered = at(half, "    enter_initial_configuration(sm);\n");
    let init = at(half, "_init_impl(idler_t *sm");
    assert!(
        entered < init,
        "the entry half is defined before the `_init` that calls it:\n{half}"
    );
    assert!(
        half[init..].contains("    idler_init_enter(sm);\n"),
        "{half}"
    );
}

/// docs/adr/0045: a component's session is named by the component, `mtc` or
/// the PTC's id, as on every other backend: the construction is given the
/// context's name as the id a host gives a root, and a name it refuses ends
/// the run in error, with the reason, before the context is set or the initial
/// configuration entered. The header declares the entry a host gives a root's
/// id through, which the same construction serves. What the names are when the
/// suite runs is held by
/// `backends/c/tests/integration/test_a_test_suite_runs_through_its_runner.c`.
#[test]
fn a_component_names_its_session_after_itself() {
    for (name, _, _) in COMPONENTS {
        let source = file(name, &format!("{name}_sm.c"));
        let start = &source[at(&source, &format!("{name}_component_start("))..];
        let constructed = at(
            start,
            &format!("{name}_init_construct(sm, false, sce_test_context_name(context)"),
        );
        let refused = at(
            start,
            "        sce_test_context_error(context, sce_start_refusal_text(refused));\n        \
             return;\n",
        );
        let set = at(start, "sm->test_context = *context;");
        let entered = at(start, &format!("{name}_init_enter(sm);"));
        assert!(
            constructed < refused && refused < set && set < entered,
            "{name}: a refused name ends the start before the context is set:\n{start}"
        );
        assert!(
            !start[..set].contains(&format!("(void){name}_init_construct(")),
            "{name}: the construction's refusal is read, not dropped"
        );
        let header = file(name, &format!("{name}_sm.h"));
        assert!(
            header.contains(&format!(
                "sce_start_refusal_t {name}_init_with_session_id({name}_t *sm,\n"
            )),
            "{name}'s header declares the start under a given id:\n{header}"
        );
        assert!(
            header.contains("\n#include \"sce/session_id.h\"\n"),
            "{name}'s header includes what the start answers"
        );
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks whether the snapshot's head is a value of its template's
/// record under its record's event, by the name every template of the record
/// gives its type, and whether the template's `matches` holds of it, through
/// a function of the machine's own that binds the template's arguments; the
/// machine includes the template unit's header. A receive any, a timeout and a
/// done read the snapshot alone. A done branch is not taken on the event it
/// names.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "if (sce_test_receives(&sm->test_context, \"sut\", \"job.reply\", reply_ok_value_type(), \
         sizeof(reply_ok_value_t), asker_test_matches_1, sm)) {",
    );
    let timeout = at(
        &asker,
        "if (sce_test_context_timed_out(&sm->test_context, \"reply\")) {",
    );
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("if (event == ASKER_EVENT_SCE_TEST_SNAPSHOT)"));
    let matcher = at(
        &asker,
        "SCE_SM_FN static bool asker_test_matches_1(const void *value, const void *closure) {\n    \
         const asker_t *sm = (const asker_t *)closure;\n    (void)sm;\n    \
         return reply_ok_matches((const reply_ok_value_t *)value);\n}",
    );
    assert!(
        matcher < receive,
        "the function the guard names is defined before the guard"
    );
    for unit in ["reply_ok", "request_of"] {
        assert!(
            asker.contains(&included(unit)),
            "the asker includes the header of {unit}, which its text names"
        );
    }

    let coordinator = text("coordinator");
    assert!(coordinator.contains("sce_test_context_done(&sm->test_context, \"answerer\")"));
    assert!(
        !coordinator.contains("if (event == COORDINATOR_EVENT_DONE_INVOKE_ANSWERER)"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "sce_test_receives(&sm->test_context, \"peer\", \"job.request\", \
         any_request_value_type(), sizeof(any_request_value_t), coordinator_test_matches_1, sm)"
    ));
    for unit in ["any_request", "reply_of"] {
        assert!(coordinator.contains(&included(unit)), "{unit}");
    }

    let idler = text("idler");
    assert!(idler.contains("if (sce_test_context_snapshot_present(&sm->test_context, \"peer\")) {"));
    assert!(
        !idler.contains("_test_matches_"),
        "a component that matches with no template defines no matcher:\n{idler}"
    );
}

/// A component whose one branch is guarded by `cond`, of the variables
/// `armed` and `count`.
fn guarded(dir: &Path, cond: &str) -> PathBuf {
    let runtime = runtime_dir();
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="guarded"
       datamodel="sce-static" initial="waiting">
  <sce:import kind="test-port-type" src="{runtime}/exchange_port.scxml" as="ExchangePort"/>
  <sce:test-port id="sut" type="ExchangePort"/>
  <datamodel>
    <data id="armed" sce:type="bool" expr="true"/>
    <data id="count" sce:type="uint8" expr="1"/>
  </datamodel>
  <state id="waiting">
    <transition sce:receive="sut" cond="{cond}" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        runtime = runtime.display()
    );
    let path = dir.join("guarded.scxml");
    std::fs::write(&path, component).expect("the component is written");
    path
}

/// A branch's guard is its `cond` and then its match, so a `cond` that is
/// false never asks the snapshot (P2: the guard is evaluated before the
/// matching), and a branch whose guard is its match alone is still guarded.
/// A `cond` that can fail is the head of its `if` on C, its verdict the `if`
/// the match is the `if` of: the snapshot is asked only of a `cond` that held.
#[test]
fn a_branch_with_a_cond_is_guarded_by_the_cond_and_then_its_match() {
    let dir = tempfile::tempdir().expect("a directory");
    let code = text_at(&guarded(dir.path(), "armed"));
    assert!(
        code.contains(") && (sce_test_context_snapshot_present(&sm->test_context, \"sut\"))) {"),
        "the match follows the cond:\n{code}"
    );
    assert!(
        text("idler")
            .contains("if (sce_test_context_snapshot_present(&sm->test_context, \"peer\")) {"),
        "a branch with no cond is guarded by its match"
    );

    let dir = tempfile::tempdir().expect("a directory");
    let code = text_at(&guarded(dir.path(), "count + 1 &gt; 1"));
    let head = at(&code, "bool sce_cond_1_ = false; ");
    let guard = at(
        &code,
        " if (sce_cond_1_) if (sce_test_context_snapshot_present(&sm->test_context, \"sut\")) {",
    );
    assert!(
        head < guard,
        "a cond that can fail is asked first, and the match only where it held:\n{code}"
    );
    assert!(
        !code.contains(") && (sce_test_context_snapshot_present("),
        "the head of an `if` is no expression to join:\n{code}"
    );
}

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch, in the
/// block of its content. A receive branch whose content reads the message
/// removes it into a message of its own, takes it as its template's record by
/// the record's name, and copies each field into the member of the payload
/// channel its record's event is held in, which its content reads as
/// `_event.data`, so its content needs no check of a delivery's payload. A
/// removal the run refused ends the block. A branch whose document writes no
/// content has the statement alone as its content.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    let asker = text("asker");
    let read = at(
        &asker,
        "sce_test_message_t sce_test_message_; if (!sce_test_context_receive(&sm->test_context, \
         \"sut\", &sce_test_message_)) { return; }",
    );
    let checked = at(
        &asker,
        "const reply_ok_value_t *sce_test_received_ = (const reply_ok_value_t \
         *)sce_test_message_value_as(&sce_test_message_, reply_ok_value_type(), \
         sizeof(reply_ok_value_t)); if (sce_test_received_ == NULL) { return; }",
    );
    let bound = at(
        &asker,
        " sm->pending_payload.as.job_reply.job = sce_test_received_->job; \
         sm->pending_payload.as.job_reply.code = sce_test_received_->code;",
    );
    let content = at(
        &asker,
        "sce_test_context_stop_timer(&sm->test_context, \"reply\");",
    );
    assert!(read < checked && checked < bound && bound < content);
    assert!(
        !asker.contains("ASKER_PAYLOAD_SCE_TEST_SNAPSHOT"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(
        &asker,
        "sce_test_context_timeout(&sm->test_context, \"reply\");",
    );
    let failed = at(
        &asker,
        "sce_test_context_setverdict(&sm->test_context, SCE_TEST_VERDICT_FAIL, ",
    );
    assert!(timeout < failed);

    assert!(text("coordinator")
        .contains("sce_test_context_done_taken(&sm->test_context, \"answerer\");"));
    // A receive any branch with no content still takes its message, and a
    // timeout branch with none still takes its timer: each is a transition
    // with content, which the microstep runs.
    assert!(text("idler")
        .contains("if (!sce_test_context_receive(&sm->test_context, \"peer\", NULL)) { return; }"));
    assert!(
        text("ptc_keeper").contains("sce_test_context_timeout(&sm->test_context, \"patience\");")
    );
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none;
/// a send names its record's event and lends its specific template's value to
/// the test-port processor, under the name of its record's type and with the
/// record type the test log names and writes it by (§12.5); a PTC is
/// started through the host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "sce_test_context_start_timer(&sm->test_context, \"reply\", INT64_C(1000000));",
        "sce_test_context_stop_timer(&sm->test_context, \"reply\");",
        "sce_test_context_setverdict(&sm->test_context, SCE_TEST_VERDICT_PASS, \"\");",
        "no matching reply in time",
        "_host_req.processor_type = \"http://sce.dev/ext/test-port\";",
        "_host_req.event_name = \"job.request\";",
        "_host_req.target = \"sut\";",
        "const request_of_value_t sce_test_sent_ = request_of_value(sm->policy.v_job); \
         const sce_test_message_value_t sce_test_sent_message_ = sce_test_message_value(\
         sce_host_value_of(request_of_value_type(), &sce_test_sent_, sizeof(sce_test_sent_)), \
         request_of_record_type()); \
         _host_req.value = sce_test_message_value_lent(&sce_test_sent_message_);",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`:\n{asker}");
    }
    assert!(
        !asker.contains("_host_req.value = sce_host_value_none();"),
        "a test-port send carries its value:\n{asker}"
    );
    let coordinator = text("coordinator");
    assert!(
        coordinator.contains("reply_of_value(sm->policy.v_requested, sm->policy.v_ok);"),
        "coordinator sends the reply its variables name:\n{coordinator}"
    );
    // A branch that receives and sends holds the send's value in a block
    // nested in the block that holds the message it took: the two locals
    // carry two names, so neither shadows the other (`-Wshadow`).
    let received = at(&coordinator, "if (sce_test_received_ == NULL) { return; }");
    let sent = at(&coordinator, "const reply_of_value_t sce_test_sent_ = ");
    assert!(
        received < sent,
        "coordinator receives, then sends:\n{coordinator}"
    );
    assert!(text("ptc_keeper").contains(
        "sce_test_context_start_timer(&sm->test_context, \"patience\", INT64_C(10000));"
    ));
    assert!(text("idler").contains(
        "sce_test_context_setverdict(&sm->test_context, SCE_TEST_VERDICT_INCONCLUSIVE, \"\");"
    ));
}

/// P8, §12.3: a test component's `<log>` is recorded in the test log through
/// its runtime context, its label, a colon and its value as the engine's log
/// writes it, into a buffer one byte longer than a line of the log holds, so
/// a line the log cannot hold is refused by the run rather than cut here; and
/// never in the engine's log. A `<log>` that writes only a label is recorded
/// too.
#[test]
fn a_log_is_recorded_in_the_test_log() {
    let asker = text("asker");
    assert!(
        asker.contains(
            "{ char sce_log_[SCE_TEST_MAX_LOG_DETAIL + 1u]; (void)snprintf(sce_log_, \
             sizeof(sce_log_), \"%s: %llu\", \"answered\", (unsigned long \
             long)(sm->pending_payload.as.job_reply.job)); \
             sce_test_context_log(&sm->test_context, sce_log_); }"
        ),
        "the asker logs the job of the message the branch took through the context:\n{asker}"
    );
    assert!(
        !asker.contains("(void)fprintf(stderr, \"%s: %llu\\n\", \"answered\""),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("sce_test_context_log(&sm->test_context, \"asked\");"));
}

/// The unit of a template takes every parameter by value, a text as the
/// buffer of its bound: a number is handed over as the machine holds it, and
/// a text as the machine's buffer itself, not the text it holds.
#[test]
fn a_template_argument_is_handed_over_as_the_unit_takes_it() {
    let dir = tempfile::tempdir().expect("a directory");
    let profile = profile_dir();
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="reporter"
       datamodel="sce-static" initial="reporting">
  <sce:import kind="test-port-type" src="{profile}/job_port.scxml" as="JobPort"/>
  <sce:import kind="test-template" src="{profile}/job_report_of.scxml" as="JobReportOf"/>
  <sce:test-port id="sut" type="JobPort"/>
  <datamodel>
    <data id="elapsed" sce:type="uint32" expr="3"/>
    <data id="note" sce:type="string[32]" expr="'late'"/>
    <data id="code" sce:type="uint8" expr="9"/>
  </datamodel>
  <state id="reporting">
    <onentry>
      <send type="http://sce.dev/ext/test-port" target="sut"
            sce:test-template="JobReportOf" sce:template-args="elapsed note code"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        profile = profile.display()
    );
    let path = dir.path().join("reporter.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    assert!(
        code.contains(
            "job_report_of_value(sm->policy.v_elapsed, sm->policy.v_note, sm->policy.v_code);"
        ),
        "{code}"
    );
    assert!(code.contains(&included("job_report_of")), "{code}");
}

/// Writes each `(file, text)` into `dir`.
fn write_all(dir: &Path, documents: &[(&str, String)]) {
    for (file, text) in documents {
        std::fs::write(dir.join(file), text).unwrap_or_else(|e| panic!("{file} is written: {e}"));
    }
}

/// A port type that receives the event schema `schema_file` declares,
/// imported as `alias`.
fn port_of(alias: &str, schema_file: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="watched_port">
  <sce:import kind="event-schema" src="{schema_file}" as="{alias}"/>
  <sce:message type="{alias}" direction="in"/>
</scxml>
"#
    )
}

/// What a component that waits on its port `sut` for a message reads it by:
/// the record's schema as `(alias, file)` among the imported records, the
/// template that matches it, the variables it declares, the variables bound to
/// the template's parameters and the content of its receive branch.
struct Watching<'a> {
    schema: (&'a str, &'a str),
    template: &'a str,
    data: &'a str,
    args: &'a str,
    content: &'a str,
}

/// The component `name` that waits on its port `sut` for the message
/// `watching` says, importing the enum of the imported records' colors when
/// `colors`.
fn watcher(name: &str, watching: &Watching<'_>, colors: bool) -> String {
    let imports = imports_dir();
    let (alias, schema_file) = watching.schema;
    let template = watching.template;
    let data = watching.data;
    let content = watching.content;
    let bound = if watching.args.is_empty() {
        String::new()
    } else {
        format!(" sce:template-args=\"{}\"", watching.args)
    };
    let color = if colors {
        format!(
            "\n  <sce:import kind=\"enum\" src=\"{}/paint_color.scxml\" as=\"Color\"/>",
            imports.display()
        )
    } else {
        String::new()
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="{name}"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="watched_port.scxml" as="WatchedPort"/>
  <sce:import kind="test-template" src="{template}" as="Watched"/>
  <sce:import kind="event-schema" src="{imports}/{schema_file}" as="{alias}"/>{color}
  <sce:test-port id="sut" type="WatchedPort"/>
  <datamodel>
{data}
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="Watched"{bound} target="done">
      {content}
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    )
}

/// The record's unit holds a text in the buffer of its bound and the machine's
/// payload channel as a pointer, so a receive branch whose content reads a
/// message of a record with a text field copies the text into the buffer the
/// machine keeps for that field, which the build holds the bound to, and
/// points the channel's field at it; a number is assigned.
#[test]
fn a_message_with_a_text_field_its_content_reads_is_copied_on_c11() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let schema = format!("{}/card_payment.scxml", imports.display());
    write_all(
        dir.path(),
        &[
            ("watched_port.scxml", port_of("Card", &schema)),
            (
                "card_watcher.scxml",
                watcher(
                    "card_watcher",
                    &Watching {
                        schema: ("Card", "card_payment.scxml"),
                        template: &format!("{}/card_within.scxml", imports.display()),
                        data: "    <data id=\"most\" sce:type=\"uint32\" expr=\"5\"/>\n    \
                               <data id=\"paid\" sce:type=\"uint32\" expr=\"0\"/>",
                        args: "most",
                        content: "<assign location=\"paid\" expr=\"_event.data.amount\"/>",
                    },
                    false,
                ),
            ),
        ],
    );
    let code = text_at(&dir.path().join("card_watcher.scxml"));
    for needle in [
        "sce_test_receives(&sm->test_context, \"sut\", \"payment.card\", \
         card_within_value_type(), sizeof(card_within_value_t), card_watcher_test_matches_1, sm)",
        "return card_within_matches((const card_within_value_t *)value, sm->policy.v_most);",
        "SCE_STATIC_ASSERT(sizeof(sce_test_received_->last4.data) <= \
         sizeof(sm->lifted_payment_card_last4), ",
        "memcpy(sm->lifted_payment_card_last4, sce_test_received_->last4.data, \
         sizeof(sce_test_received_->last4.data)); \
         sm->pending_payload.as.payment_card.last4 = sm->lifted_payment_card_last4;",
        "sm->pending_payload.as.payment_card.amount = sce_test_received_->amount;",
    ] {
        assert!(
            code.contains(needle),
            "card_watcher holds `{needle}`:\n{code}"
        );
    }
}

/// The machine holds a value of an enum in the type the enum's unit declares,
/// so a message the content reads that holds an enum field is assigned to the
/// channel's field as it is, as a variable of an enum bound to a parameter is
/// handed to the template as it is.
#[test]
fn a_message_with_an_enum_field_its_content_reads_is_copied_on_c11() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let schema = format!("{}/order_line.scxml", imports.display());
    write_all(
        dir.path(),
        &[
            ("watched_port.scxml", port_of("OrderLine", &schema)),
            (
                "color_reader.scxml",
                watcher(
                    "color_reader",
                    &Watching {
                        schema: ("OrderLine", "order_line.scxml"),
                        template: &format!("{}/order_line_exact.scxml", imports.display()),
                        data: "    <data id=\"seen\" sce:type=\"enum:Color\" expr=\"Color.red\"/>",
                        args: "",
                        content: "<assign location=\"seen\" expr=\"_event.data.color\"/>",
                    },
                    true,
                ),
            ),
        ],
    );
    let code = text_at(&dir.path().join("color_reader.scxml"));
    for needle in [
        "return order_line_exact_matches((const order_line_exact_value_t *)value);",
        "sm->pending_payload.as.order_line.color = sce_test_received_->color;",
        "sm->pending_payload.as.order_line.quantity = sce_test_received_->quantity;",
        "memcpy(sm->lifted_order_line_sku, sce_test_received_->sku.data, \
         sizeof(sce_test_received_->sku.data));",
    ] {
        assert!(
            code.contains(needle),
            "color_reader holds `{needle}`:\n{code}"
        );
    }
}

/// The unit of an event schema holds a byte string of one entry in the buffer
/// of its bound beside its length, and the machine's channel in an array of
/// the same capacity beside its `_len`, so a receive branch whose content reads
/// a message of a record with one copies its bytes and its length, the build
/// holding the two capacities alike.
#[test]
fn a_message_with_a_byte_string_its_content_reads_is_copied_on_c11() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let schema = format!("{}/voucher_payment.scxml", imports.display());
    let template = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="any_voucher"
       sce:type="record:Voucher">
  <sce:import kind="event-schema" src="{schema}" as="Voucher"/>
  <sce:record-match>
    <sce:any-value field="voucher_code"/>
  </sce:record-match>
</scxml>
"#
    );
    write_all(
        dir.path(),
        &[
            ("watched_port.scxml", port_of("Voucher", &schema)),
            ("any_voucher.scxml", template),
            (
                "voucher_reader.scxml",
                watcher(
                    "voucher_reader",
                    &Watching {
                        schema: ("Voucher", "voucher_payment.scxml"),
                        template: "any_voucher.scxml",
                        data: "    <data id=\"code\" sce:type=\"bytes[4]\" expr=\"''\"/>",
                        args: "",
                        content: "<assign location=\"code\" expr=\"_event.data.voucher_code\"/>",
                    },
                    false,
                ),
            ),
        ],
    );
    let code = text_at(&dir.path().join("voucher_reader.scxml"));
    for needle in [
        "SCE_STATIC_ASSERT(sizeof(sce_test_received_->voucher_code.data) <= sizeof(",
        ".voucher_code, sce_test_received_->voucher_code.data, \
         sce_test_received_->voucher_code.len);",
        ".voucher_code_len = sce_test_received_->voucher_code.len;",
    ] {
        assert!(
            code.contains(needle),
            "voucher_reader holds `{needle}`:\n{code}"
        );
    }
}

/// §11.5, §11.8, §12.3: a module parameter is declared with its type and no
/// value of the document's, and the construction starts it at its type's
/// zero, which nothing reads; the component reads the run's value into it, as
/// a saved value of its type is read back, after the construction has cleared
/// the machine and the context is set and before the initial configuration is
/// entered, and does not enter it when the read is refused. Every value is
/// read before any is written. The twin of the Go and Rust tests of the same
/// name.
#[test]
fn a_module_parameter_is_read_from_the_run_before_the_machine_starts() {
    assert!(sce_build::forge::codegen_matrix::TEST_SUITE_BACKENDS.contains(&Language::C11));
    let source = file("param_asker", "param_asker_sm.c");
    for needle in [
        "\n#include <sce/saved_values.h>\n",
        "\n    sm->policy.v_job = 0;\n",
        "SCE_SM_FN static bool param_asker_component_accept_module_parameters(void *user_data,",
        "\n    param_asker_t *sm = (param_asker_t *)user_data;\n",
        "\n    sce_jv_t raw1;\n    uint32_t read1;\n",
        "\n    if (!sce_test_module_parameters_of(given, \"job\", &raw1, err) ||\n        \
         !sce_saved_read_uint32(raw1, \"job\", &read1, err)) {\n        return false;\n    }\n",
        "\n    sm->policy.v_job = read1;\n    return true;\n}",
    ] {
        assert!(
            source.contains(needle),
            "param_asker holds `{needle}`:\n{source}"
        );
    }
    let accept = &source[at(
        &source,
        "_component_accept_module_parameters(void *user_data,",
    )..];
    let read_back = at(accept, "sce_saved_read_uint32(raw1, \"job\", &read1, err)");
    let written = at(accept, "sm->policy.v_job = read1;");
    assert!(
        read_back < written,
        "the value is read before it is written"
    );
    let start = &source[at(&source, "param_asker_component_start(")..];
    let constructed = at(
        start,
        "param_asker_init_construct(sm, false, sce_test_context_name(context)",
    );
    let set = at(start, "sm->test_context = *context;");
    let read = at(
        start,
        "if (!sce_test_context_read_module_parameters(&sm->test_context, \
         param_asker_component_accept_module_parameters,",
    );
    let entered = at(start, "param_asker_init_enter(sm);");
    assert!(
        constructed < set && set < read && read < entered,
        "the parameters are read once the context is set and before the entry"
    );
    assert!(
        start[read..entered].contains("return;"),
        "a refused read leaves the initial configuration unentered"
    );
    // The variable that is no module parameter starts at its `expr`.
    assert!(
        source.contains("\n    sm->policy.v_answered = "),
        "{source}"
    );
    let asker = file("asker", "asker_sm.c");
    assert!(
        !asker.contains("_component_accept_module_parameters"),
        "a component with no module parameter reads none"
    );
}

/// §11.8: a byte string module parameter is read back held to its bound, as a
/// restored byte string is, and a text by the reader of its bounded buffer;
/// each starts at the empty value of the buffer of its bound.
#[test]
fn a_bounded_module_parameter_is_read_back_held_to_its_bound() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="keyed"
       datamodel="sce-static" initial="checking">
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="key" sce:type="bytes[2]" sce:module-parameter="true"/>
    <data id="label" sce:type="string[8]" sce:module-parameter="true"/>
  </datamodel>
  <state id="checking">
    <onentry>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;
    let path = dir.path().join("keyed.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    for needle in [
        "\n    sm->policy.v_key = (sce_static_bytes_2_t){ { 0 }, 0 };\n",
        "\n    sm->policy.v_label = (sce_static_string_8_t){ { 0 } };\n",
        "\n    sce_static_bytes_2_t read1;\n",
        "\n    sce_static_string_8_t read2;\n",
        "!sce_saved_read_bytes(raw1, \"key\", (read1).data, 2u, &(read1).len, err)",
        "!sce_saved_read_string_bounded(raw2, \"label\", (read2).data, 8u, err)",
        "\n    sm->policy.v_key = read1;\n    sm->policy.v_label = read2;\n",
    ] {
        assert!(code.contains(needle), "keyed holds `{needle}`:\n{code}");
    }
}

/// §11.8: a module parameter of an enum is read through the enum's saved
/// form, the `_saved_read` its machine type declares beside it whether or not
/// a saved state of the machine can be written. Zero may name no variant of
/// the enum, so the variable starts at the variant its document declares
/// first, which nothing reads before the run's value replaces it.
#[test]
fn an_enum_module_parameter_is_read_through_the_enums_saved_form() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="color_picker"
       datamodel="sce-static" initial="picking">
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="tint" sce:type="enum:Color" sce:module-parameter="true"/>
  </datamodel>
  <state id="picking">
    <onentry>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports_dir().display()
    );
    let path = dir.path().join("color_picker.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    for needle in [
        "static inline bool paint_color_enum_t_saved_read(sce_jv_t v, const char *what, \
         paint_color_enum_t *out, sce_saved_error_t *err) {",
        "\n    sm->policy.v_tint = PAINT_COLOR_RED;\n",
        "\n    paint_color_enum_t read1;\n",
        "!paint_color_enum_t_saved_read(raw1, \"tint\", &read1, err)",
        "\n    sm->policy.v_tint = read1;\n",
    ] {
        assert!(
            code.contains(needle),
            "color_picker holds `{needle}`:\n{code}"
        );
    }
}

/// docs/SCE_TEST_PROFILE.md §12.3: the component the runtime drives starts
/// its machine itself, and a C11 machine that declares `<sce:action>` is
/// initialized with the table of acts its host gives, which the component is
/// given by nothing, so C11 refuses such a component by name.
#[test]
fn a_component_that_declares_a_host_action_is_refused_on_c11() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="beeper"
       datamodel="sce-static" initial="beeping">
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="beeps" sce:type="uint32" expr="1"/>
  </datamodel>
  <state id="beeping">
    <onentry>
      <sce:action name="showBeeps" effect="notice">
        <sce:arg name="count" expr="beeps"/>
      </sce:action>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;
    let path = dir.path().join("beeper.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let detail = refused(&path, "beeper");
    assert!(
        detail.starts_with("test-component 'beeper' declares <sce:action>"),
        "{detail}"
    );
    assert!(
        detail.contains("the table of acts `<machine>_init_with_actions` takes"),
        "{detail}"
    );
}

/// docs/adr/0092, section 7: a component whose template reaches a union is
/// refused on C11, which generates no unit of that template, by the union's
/// own refusal, as on C++, Go and Python.
#[test]
fn a_component_whose_template_reaches_a_union_is_refused_on_c11() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let port = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="order_port">
  <sce:import kind="event-schema" src="{imports}/order_placed.scxml" as="OrderPlaced"/>
  <sce:message type="OrderPlaced" direction="in"/>
</scxml>
"#,
        imports = imports.display()
    );
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="order_watcher"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="order_port.scxml" as="OrderPort"/>
  <sce:import kind="test-template" src="{imports}/order_placed_match.scxml" as="OrderPlacedMatch"/>
  <sce:test-port id="sut" type="OrderPort"/>
  <datamodel>
    <data id="limit" sce:type="uint16" expr="3"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="OrderPlacedMatch"
                sce:template-args="limit" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    );
    std::fs::write(dir.path().join("order_port.scxml"), port).expect("the port type is written");
    let path = dir.path().join("order_watcher.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let detail = refused(&path, "order_watcher");
    assert!(detail.contains("`union:Payment`: "), "{detail}");
    assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
}
