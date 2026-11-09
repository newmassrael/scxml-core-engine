// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in Python (docs/SCE_TEST_PROFILE.md §11.9,
//! §12.3, §12.6 slice 6): the statechart machine of the document, each branch
//! a transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile, a `<log>` and a send as calls on the component's runtime context,
//! a PTC through the host invoker door; and beside the machine the component
//! the profile's runtime drives, which builds the machine through the module's
//! factory and sets its context on the policy before the machine starts. The
//! twin of `a_test_component_is_generated_in_go.rs`, over the same fixtures.
//!
//! A machine imports the module of each template it names as a sibling
//! (`from . import <document> as sce_<document>`), as it imports an algorithm.
//! Python holds a text, a byte string and an enum of a received message where
//! the record's unit holds them, so it copies a message the content reads that
//! Rust refuses, as Kotlin and Go do. A module parameter is read from the run
//! after the machine is built and before it starts, as on Rust, C++, Kotlin and
//! Go. What Python refuses is held here too: a component that declares
//! `<sce:action>`, and one whose template reaches a union, which Python does
//! not lower.
//!
//! What the generated components do when they run is held by
//! `backends/python/tests/integration/static_datamodel/test_a_test_component_runs_through_its_doors.py`,
//! which imports them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_python.sh`) and runs them through
//! `sce_runtime.testprofile.TestCaseRun`.

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

/// Every component of the runtime's end-to-end test that Python generates,
/// each with the name its machine is spelled by.
const COMPONENTS: [(&str, &str); 6] = [
    ("asker", "Asker"),
    ("coordinator", "Coordinator"),
    ("answerer", "Answerer"),
    ("ptc_keeper", "PtcKeeper"),
    ("idler", "Idler"),
    ("param_asker", "ParamAsker"),
];

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::Python),
        Language::Python,
    )
    .map(|output| output.files)
}

/// The Python files of the component `name`.
fn files(name: &str) -> Vec<(String, String)> {
    generate(&runtime_dir().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on Python: {e:?}"))
}

/// The whole Python text of the component `name`.
fn text(name: &str) -> String {
    files(name)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole Python text of the document at `path`, which generates.
fn text_at(path: &Path) -> String {
    generate(path)
        .unwrap_or_else(|e| panic!("{} generates on Python: {e:?}", path.display()))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`:\n{haystack}"))
}

/// The import of the generated module of the document `snake`, under the alias
/// every Python unit imports a sibling module by.
fn imported(snake: &str) -> String {
    format!("\nfrom . import {snake} as sce_{snake}\n")
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on Python: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Python: {other:?}"),
        },
        other => panic!("{name} on Python: {other:?}"),
    }
}

/// The detail of the refusal by name of the document at `path`.
fn refused(path: &Path, name: &str) -> String {
    let Err(err) = generate(path) else {
        panic!("{} generated on Python", path.display());
    };
    unsupported(name, err)
}

/// §12.3: each component is its machine's module, and beside the machine the
/// component the runtime drives: the component builds the machine through the
/// module's factory, under a session the component names, sets the runtime
/// context on its policy, and registers the two doors on it before it starts
/// it and delivers the snapshot event.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for (name, machine) in COMPONENTS {
        let names: Vec<String> = files(name).into_iter().map(|(file, _)| file).collect();
        assert_eq!(names, [format!("{name}_sm.py")]);
        let code = text(name);
        for needle in [
            "\nimport sce_runtime.testprofile as _sce_testprofile\n".to_string(),
            "self._sce_test_context: Optional[_sce_testprofile.ComponentContext] = None"
                .to_string(),
            format!("\nclass {machine}Component:\n"),
            "    __test__ = False\n".to_string(),
            format!("self._engine: Optional[Engine[{machine}State, {machine}Event]] = None"),
            "def start(self, context: _sce_testprofile.ComponentContext) -> None:".to_string(),
            "engine = create_engine(session_id=context.name)".to_string(),
            "engine.policy._sce_test_context = context".to_string(),
            "engine.register_event_processor(\"http://sce.dev/ext/test-port\", send)".to_string(),
            "raise RuntimeError(\"a test-port send carries its template's value\")".to_string(),
            "context.send(request.target, _sce_testprofile.Message(event=request.event_name, \
             test_value=request.value))"
                .to_string(),
            "engine.register_invoker(\"http://sce.dev/ext/test-component\", invoke)".to_string(),
            "context.create(event.start.invoke_id)".to_string(),
            "context.kill(event.cancel.invoke_id)".to_string(),
            format!("self._engine.send_event({machine}Event.SCE_TEST_SNAPSHOT)"),
            "return self._engine is not None and self._engine.reached_final".to_string(),
            format!("    \"{machine}Component\",\n"),
        ] {
            assert!(code.contains(&needle), "{name} holds `{needle}`:\n{code}");
        }
        let component = &code[at(&code, &format!("\nclass {machine}Component:\n"))..];
        let built = at(component, "engine = create_engine(session_id=context.name)");
        let set = at(component, "engine.policy._sce_test_context = context");
        let served = at(component, "engine.register_event_processor(");
        let invoked = at(component, "engine.register_invoker(");
        let started = at(component, "\n        engine.initialize()\n");
        let kept = at(component, "\n        self._engine = engine\n");
        assert!(
            built < set && set < served && served < invoked && invoked < started && started < kept,
            "{name}: the machine is built and its policy set on the context, and the doors are \
             registered on it before it starts"
        );
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks whether the snapshot's head is of its template's type under
/// its record's event and calls the template's `matches`, both named through
/// the template unit's module, which the machine imports; a receive any, a
/// timeout and a done read the snapshot alone. A done branch is not taken on
/// the event it names.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "_sce_testprofile.receives(self._sce_test_context, \"sut\", \"job.reply\", \
         sce_reply_ok.Value, lambda _sce_message: sce_reply_ok.matches(_sce_message))",
    );
    let timeout = at(&asker, "self._sce_test_context.timed_out(\"reply\")");
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("if event == AskerEvent.SCE_TEST_SNAPSHOT and ("));
    for unit in ["reply_ok", "request_of"] {
        assert!(
            asker.contains(&imported(unit)),
            "the asker imports the module of {unit}, which its text names"
        );
    }

    let coordinator = text("coordinator");
    assert!(coordinator.contains("self._sce_test_context.done(\"answerer\")"));
    assert!(
        !coordinator.contains("if event == CoordinatorEvent.DONE_INVOKE_ANSWERER"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "_sce_testprofile.receives(self._sce_test_context, \"peer\", \"job.request\", \
         sce_any_request.Value, lambda _sce_message: sce_any_request.matches(_sce_message))"
    ));
    for unit in ["any_request", "reply_of"] {
        assert!(coordinator.contains(&imported(unit)), "{unit}");
    }

    let idler = text("idler");
    assert!(idler.contains("self._sce_test_context.snapshot_present(\"peer\")"));
    assert!(
        !idler.contains("\nfrom . import "),
        "a component that names no template imports no template's module:\n{idler}"
    );
}

/// A branch's guard is its `cond` and then its match, joined by Python's
/// `and`, so a `cond` that is false never asks the snapshot (P2: the guard is
/// evaluated before the matching), and a branch whose guard is its match alone
/// is still guarded.
#[test]
fn a_branch_with_a_cond_is_guarded_by_the_cond_and_then_its_match() {
    let dir = tempfile::tempdir().expect("a directory");
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
  </datamodel>
  <state id="waiting">
    <transition sce:receive="sut" cond="armed" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        runtime = runtime.display()
    );
    let path = dir.path().join("guarded.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    assert!(
        code.contains(") and (self._sce_test_context.snapshot_present(\"sut\"))"),
        "the match follows the cond:\n{code}"
    );
    assert!(
        !code.contains(") && ("),
        "Python's conjunction is `and`:\n{code}"
    );
    assert!(
        text("idler").contains(
            "if event == IdlerEvent.SCE_TEST_SNAPSHOT and \
             (self._sce_test_context.snapshot_present(\"peer\")):"
        ),
        "a branch with no cond is guarded by its match"
    );
}

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch. A
/// receive branch whose content reads the message holds it in the payload
/// attribute of its record's event, which its content reads as `_event.data`,
/// so its content needs no check of a delivery's payload. A branch whose
/// document writes no content has the statement alone as its content.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    let asker = text("asker");
    let read = at(
        &asker,
        "_sce_message = self._sce_test_context.receive(\"sut\")",
    );
    let checked = at(
        &asker,
        "if not isinstance(_sce_message.value, sce_reply_ok.Value):",
    );
    let bound = at(
        &asker,
        "self._pending_job_reply_payload = AskerJobReplyPayload(job=_sce_message.value.job, \
         code=_sce_message.value.code)",
    );
    let content = at(&asker, "self._sce_test_context.stop_timer(\"reply\")");
    assert!(read < checked && checked < bound && bound < content);
    assert!(
        !asker.contains("if self._pending_job_reply_payload is None:"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(&asker, "self._sce_test_context.timeout(\"reply\")");
    let failed = at(
        &asker,
        "self._sce_test_context.setverdict(_sce_testprofile.Verdict.FAIL, ",
    );
    assert!(timeout < failed);

    assert!(text("coordinator").contains("self._sce_test_context.done_taken(\"answerer\")"));
    // A receive any branch with no content still takes its message, and a
    // timeout branch with none still takes its timer: each is a transition
    // with content, which the microstep runs, and no block to run after it.
    let idler = text("idler");
    assert!(
        idler.contains(
            "                self._sce_test_context.receive(\"peer\")\n                return\n"
        ),
        "{idler}"
    );
    assert!(text("ptc_keeper").contains("self._sce_test_context.timeout(\"patience\")"));
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none;
/// a send names its record's event and carries its specific template's value
/// to the test-port processor; a PTC is started through the host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "self._sce_test_context.start_timer(\"reply\", 1000000)",
        "self._sce_test_context.stop_timer(\"reply\")",
        "self._sce_test_context.setverdict(_sce_testprofile.Verdict.PASS, \"\")",
        "no matching reply in time",
        "processor_type=\"http://sce.dev/ext/test-port\",",
        "\"job.request\"",
        "target=\"sut\",",
        "invoke_id=engine.current_event_invoke_id,",
        "value=_sce_testprofile.message_value(sce_request_of.value(self.v_job), \
         sce_request_of.TYPE, sce_request_of.canonical),",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`:\n{asker}");
    }
    let coordinator = text("coordinator");
    for needle in [
        "value=_sce_testprofile.message_value(sce_reply_of.value(self.v_requested, self.v_ok), \
         sce_reply_of.TYPE, sce_reply_of.canonical),",
        "processor_type=\"http://sce.dev/ext/test-component\",",
        "invoke_id=\"answerer\",",
    ] {
        assert!(
            coordinator.contains(needle),
            "coordinator holds `{needle}`:\n{coordinator}"
        );
    }
    assert!(text("ptc_keeper").contains("self._sce_test_context.start_timer(\"patience\", 10000)"));
    assert!(text("idler").contains(
        "self._sce_test_context.setverdict(_sce_testprofile.Verdict.INCONCLUSIVE, \"\")"
    ));
}

/// P8, §12.3: a test component's `<log>` is recorded in the test log through
/// its runtime context, its label, a colon and its value as the engine's log
/// shows it (`_sce_log_name`), and never in the engine's log; a `<log>` that
/// writes only a label is recorded too.
#[test]
fn a_log_is_recorded_in_the_test_log() {
    let asker = text("asker");
    assert!(
        asker.contains(
            "self._sce_test_context.log(\"answered: \" + \
             str(_sce_log_name(self._pending_job_reply_payload.job)))"
        ),
        "the asker logs the job of the message the branch took through the context:\n{asker}"
    );
    assert!(
        !asker.contains("self.log_hook(\"answered\""),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("self._sce_test_context.log(\"asked\")"));
}

/// The unit of a template takes every parameter by value, a number and a
/// text alike: the variables bound to its parameters are handed over as the
/// machine holds them.
#[test]
fn a_template_argument_is_handed_over_as_the_machine_holds_it() {
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
            "value=_sce_testprofile.message_value(\
             sce_job_report_of.value(self.v_elapsed, self.v_note, self.v_code), \
             sce_job_report_of.TYPE, sce_job_report_of.canonical),"
        ),
        "{code}"
    );
    assert!(code.contains(&imported("job_report_of")), "{code}");
}

/// A component over a record with a text field, whose receive branch
/// matches with a template that takes a number and whose content reads the
/// message.
fn card_watcher(dir: &Path) -> PathBuf {
    let imports = imports_dir();
    let port = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="card_port">
  <sce:import kind="event-schema" src="{imports}/card_payment.scxml" as="Card"/>
  <sce:message type="Card" direction="in"/>
</scxml>
"#,
        imports = imports.display()
    );
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="card_watcher"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="card_port.scxml" as="CardPort"/>
  <sce:import kind="test-template" src="{imports}/card_within.scxml" as="CardWithin"/>
  <sce:import kind="event-schema" src="{imports}/card_payment.scxml" as="Card"/>
  <sce:test-port id="sut" type="CardPort"/>
  <datamodel>
    <data id="most" sce:type="uint32" expr="5"/>
    <data id="paid" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="CardWithin"
                sce:template-args="most" target="done">
      <assign location="paid" expr="_event.data.amount"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    );
    std::fs::write(dir.join("card_port.scxml"), port).expect("the port type is written");
    let path = dir.join("card_watcher.scxml");
    std::fs::write(&path, component).expect("the component is written");
    path
}

/// The machine's payload dataclass and the record's unit hold a text in the
/// same `str`, so a receive branch whose content reads a message of a record
/// with a text field copies it on Python, as on Kotlin and Go.
#[test]
fn a_message_with_a_text_field_its_content_reads_is_copied_on_python() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = card_watcher(dir.path());
    let code = text_at(&path);
    for needle in [
        "_sce_testprofile.receives(self._sce_test_context, \"sut\", \"payment.card\", \
         sce_card_within.Value, lambda _sce_message: sce_card_within.matches(_sce_message, \
         self.v_most))",
        "if not isinstance(_sce_message.value, sce_card_within.Value):",
        "self._pending_payment_card_payload = CardWatcherPaymentCardPayload(\
         last4=_sce_message.value.last4, amount=_sce_message.value.amount)",
    ] {
        assert!(
            code.contains(needle),
            "card_watcher holds `{needle}`:\n{code}"
        );
    }
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

/// A component that waits on its port `sut` for a message its template
/// `template` matches, given the variable `wanted` of type `wanted_type`
/// declared at `wanted_expr`.
fn color_watcher(template: &str, wanted_type: &str, wanted_expr: &str) -> String {
    let imports = imports_dir();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="color_watcher"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="watched_port.scxml" as="WatchedPort"/>
  <sce:import kind="test-template" src="{template}.scxml" as="Watched"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:test-port id="sut" type="WatchedPort"/>
  <datamodel>
    <data id="wanted" sce:type="{wanted_type}" expr="{wanted_expr}"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="Watched"
                sce:template-args="wanted" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    )
}

/// A component that waits on its port `sut` for the exact order line and
/// keeps the line's color, which its content reads from the message.
fn color_reader() -> String {
    let imports = imports_dir();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="color_reader"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="watched_port.scxml" as="WatchedPort"/>
  <sce:import kind="test-template" src="{imports}/order_line_exact.scxml" as="ExactLine"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:import kind="event-schema" src="{imports}/order_line.scxml" as="OrderLine"/>
  <sce:test-port id="sut" type="WatchedPort"/>
  <datamodel>
    <data id="seen" sce:type="enum:Color" expr="Color.red"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="ExactLine" target="done">
      <assign location="seen" expr="_event.data.color"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    )
}

/// A template over an order line whose color is its one formal parameter.
fn line_of_color() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="line_of_color"
       sce:type="record:OrderLine">
  <sce:import kind="event-schema" src="{imports}/order_line.scxml" as="OrderLine"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <datamodel>
    <data id="color" sce:type="enum:Color" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:any-value field="sku"/>
    <sce:any-value field="quantity"/>
    <sce:specific field="color" param="color"/>
  </sce:record-match>
</scxml>
"#,
        imports = imports_dir().display()
    )
}

/// The port type of an order line, written into `dir` as `watched_port.scxml`.
fn write_order_line_port(dir: &Path) {
    let schema = format!("{}/order_line.scxml", imports_dir().display());
    write_all(
        dir,
        &[("watched_port.scxml", port_of("OrderLine", &schema))],
    );
}

/// docs/SCE_TEST_PROFILE.md §11.9: the machine holds a value of an enum in its
/// own `IntEnum` and a template's unit takes the type the enum document's unit
/// declares, so a variable of an enum bound to a parameter crosses into the
/// variant of the same declared name, as a value handed to an algorithm does,
/// and the machine imports the enum's unit, whose type the crossing names.
#[test]
fn an_enum_argument_crosses_into_the_templates_enum() {
    let dir = tempfile::tempdir().expect("a directory");
    write_order_line_port(dir.path());
    write_all(
        dir.path(),
        &[
            ("line_of_color.scxml", line_of_color()),
            (
                "color_watcher.scxml",
                color_watcher("line_of_color", "enum:Color", "Color.blue"),
            ),
        ],
    );
    let code = text_at(&dir.path().join("color_watcher.scxml"));
    assert!(
        code.contains("sce_line_of_color.matches(_sce_message, {ColorWatcherColorEnum."),
        "the argument crosses where the template is called: {code}"
    );
    assert!(code.contains(": sce_paint_color.PaintColor."), "{code}");
    assert!(code.contains("}[self.v_wanted])"), "{code}");
    assert!(code.contains(&imported("paint_color")), "{code}");
    assert!(code.contains(&imported("line_of_color")), "{code}");
}

/// A message the content reads that holds an enum field is copied into the
/// machine's own enum by the variant's declared name, read as a saved value of
/// that type is, where Rust and C++ refuse it by name.
#[test]
fn a_message_with_an_enum_field_its_content_reads_is_copied_on_python() {
    let dir = tempfile::tempdir().expect("a directory");
    write_order_line_port(dir.path());
    write_all(dir.path(), &[("color_reader.scxml", color_reader())]);
    let code = text_at(&dir.path().join("color_reader.scxml"));
    for needle in [
        "_sce_testprofile.receives(self._sce_test_context, \"sut\", \"order.line\", \
         sce_order_line_exact.Value, lambda _sce_message: \
         sce_order_line_exact.matches(_sce_message))",
        "self._pending_order_line_payload = ColorReaderOrderLinePayload(\
         sku=_sce_message.value.sku, quantity=_sce_message.value.quantity, \
         color=ColorReaderColorEnum.from_saved(_sce_message.value.color.declared_name(), \
         \"color\"))",
        "class ColorReaderColorEnum(IntEnum):",
        "    def from_saved(cls, value, what):",
    ] {
        assert!(
            code.contains(needle),
            "color_reader holds `{needle}`:\n{code}"
        );
    }
}

/// An optional value of an enum has no crossing into a template's unit yet,
/// so a variable of one bound to a parameter is refused by name, on Python as
/// on the other backends.
#[test]
fn an_optional_enum_argument_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let schema = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="tinted"
       sce:event-name="tinted">
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <datamodel>
    <data id="tint" sce:type="optional&lt;enum:Color&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
        imports = imports.display()
    );
    let template = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="tint_of"
       sce:type="record:Tinted">
  <sce:import kind="event-schema" src="tinted.scxml" as="Tinted"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <datamodel>
    <data id="tint" sce:type="optional&lt;enum:Color&gt;" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:specific field="tint" param="tint"/>
  </sce:record-match>
</scxml>
"#,
        imports = imports.display()
    );
    write_all(
        dir.path(),
        &[
            ("tinted.scxml", schema),
            ("watched_port.scxml", port_of("Tinted", "tinted.scxml")),
            ("tint_of.scxml", template),
            (
                "color_watcher.scxml",
                color_watcher("tint_of", "optional&lt;enum:Color&gt;", "null"),
            ),
        ],
    );
    let detail = refused(&dir.path().join("color_watcher.scxml"), "color_watcher");
    assert!(
        detail.contains("an optional value of the enum `Color` bound to a parameter of a test"),
        "{detail}"
    );
}

/// §11.5, §11.8, §12.3: a module parameter is declared with its type and no
/// value of the document's, at its type's zero, which nothing reads; the
/// component reads the run's value into it, as a saved value of its type is
/// read back, after the machine is built and its policy set on the context and
/// before the doors are registered and the machine starts, and does not start
/// the machine when the read is refused. Every value is read before any is
/// written. The twin of the Go, Kotlin and Rust tests of the same name.
#[test]
fn a_module_parameter_is_read_from_the_run_before_the_machine_starts() {
    assert!(sce_build::forge::codegen_matrix::TEST_SUITE_BACKENDS.contains(&Language::Python));
    let code = text("param_asker");
    for needle in [
        "        v_job: int = 0\n",
        "        v_answered: int = 0\n",
        "    def _sce_accept_module_parameters(self, given: \
         _sce_testprofile.ModuleParameters) -> None:\n",
        "        read_1 = sce_saved.saved_read_uint32(given.of(\"job\"), \"job\")\n",
        "        self.v_job = read_1\n",
        "        if not context.read_module_parameters(\
         engine.policy._sce_accept_module_parameters):\n            return\n",
    ] {
        assert!(
            code.contains(needle),
            "param_asker holds `{needle}`:\n{code}"
        );
    }
    let accept = &code[at(&code, "def _sce_accept_module_parameters(")..];
    let read_back = at(accept, "read_1 = sce_saved.saved_read_uint32(");
    let written = at(accept, "self.v_job = read_1");
    assert!(
        read_back < written,
        "the value is read before it is written"
    );
    let component = &code[at(&code, "\nclass ParamAskerComponent:\n")..];
    let built = at(component, "engine = create_engine(session_id=context.name)");
    let set = at(component, "engine.policy._sce_test_context = context");
    let read = at(component, "if not context.read_module_parameters(");
    let served = at(component, "engine.register_event_processor(");
    let started = at(component, "\n        engine.initialize()\n");
    let kept = at(component, "\n        self._engine = engine\n");
    assert!(
        built < set && set < read && read < served && served < started && started < kept,
        "the parameters are read after the policy is set on the context and before the \
         machine starts:\n{component}"
    );
    assert!(
        !text("asker").contains("_sce_accept_module_parameters"),
        "a component with no module parameter reads none"
    );
}

/// §11.8: a byte string module parameter is read back held to its bound, as a
/// restored byte string is, and a text by the reader of its bounded type; each
/// starts at its type's zero.
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
        "        v_key: bytes = b\"\"\n",
        "        v_label: str = \"\"\n",
        "read_1 = sce_saved.saved_read_bytes(given.of(\"key\"), \"key\", 2)",
        "read_2 = sce_saved.saved_read_bounded_string(given.of(\"label\"), \"label\", 8)",
        "self.v_key = read_1",
        "self.v_label = read_2",
    ] {
        assert!(code.contains(needle), "keyed holds `{needle}`:\n{code}");
    }
}

/// §11.8: a module parameter of an enum is read through the enum's saved
/// form, the `from_saved` its machine type declares whether or not a saved
/// state of the machine can be written. The machine's `IntEnum` has no zero of
/// its document's, so the variable starts at the variant its document declares
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
        "\nclass ColorPickerColorEnum(IntEnum):\n",
        "    def from_saved(cls, value, what):\n",
        "        v_tint: ColorPickerColorEnum = ColorPickerColorEnum.RED\n",
        "read_1 = ColorPickerColorEnum.from_saved(given.of(\"tint\"), \"tint\")",
        "self.v_tint = read_1",
    ] {
        assert!(
            code.contains(needle),
            "color_picker holds `{needle}`:\n{code}"
        );
    }
}

/// docs/SCE_TEST_PROFILE.md §12.3: the component the runtime drives builds its
/// machine itself, and a Python policy of a machine that declares
/// `<sce:action>` is constructed with a host the component is given by
/// nothing, so Python refuses such a component by name.
#[test]
fn a_component_that_declares_a_host_action_is_refused_on_python() {
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
        detail.contains("a parameter of the Python policy's constructor"),
        "{detail}"
    );
}

/// docs/adr/0092, section 7: a component whose template reaches a union is
/// refused on Python, which generates no unit of that template, by the union's
/// own refusal, as on C++ and Go.
#[test]
fn a_component_whose_template_reaches_a_union_is_refused_on_python() {
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
