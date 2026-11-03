// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A procedure (`sce:kind="procedure"`, states that send to services, transitions on the answers,
// assigns and done data), written for Swift: a `final class` that adopts the runtime's
// `ProcedureStateMachine`, with the states and events as enumerations nested in it. What the
// class cannot spell is refused by name and not written as something else: an import of a kind
// that holds state, a state or an event Swift reserves, two names that would be one case, and a
// variable named for a member the class already has.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, compile_forge_from_string, DocumentLabel, ForgeCompileOptions,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

/// A procedure named `watch` with `data` as its datamodel and `body` as its states, starting
/// in `start`, written to Swift from a string.
fn swift_of_procedure(data: &str, start: &str, body: &str) -> Result<String, String> {
    let document = format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="procedure" name="watch" initial="{start}">
  <datamodel>{data}</datamodel>
  {body}
</scxml>"#
    );
    compile_forge_from_string(
        &document,
        DocumentLabel {
            identifier: "watch",
            diagnostic_label: "watch.scxml",
        },
        Language::Swift,
    )
    .map_err(|e| format!("{e:?}"))
    .map(|output| {
        output
            .files
            .into_iter()
            .next()
            .expect("a document that is written writes a file")
            .1
    })
}

const INPUT: &str = r#"<data id="level" sce:type="int32" sce:direction="in"/>"#;

#[test]
fn a_procedure_is_a_class_that_adopts_the_runtimes_state_machine() {
    let code = swift_of("tests/forge/resources/procedure_security_access.scxml")
        .expect("a procedure is written");
    assert!(
        code.contains("public final class ProcedureSecurityAccess: ProcedureStateMachine {"),
        "{code}"
    );
    assert!(code.contains("import SceForgeRuntime"), "{code}");
    assert!(
        code.contains("public func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>? {"),
        "{code}"
    );
}

#[test]
fn the_states_and_the_events_are_cases_of_enumerations_nested_in_the_class() {
    let code = swift_of("tests/forge/resources/procedure_startup_check.scxml")
        .expect("a procedure is written");
    assert!(code.contains("public enum State {"), "{code}");
    for case in [
        "checkVoltage",
        "checkTemp",
        "success",
        "failVoltage",
        "failOvertemp",
    ] {
        assert!(code.contains(&format!("case {case}\n")), "{case}: {code}");
    }
    assert!(
        code.contains("public enum Event {\n        case noEvent\n"),
        "{code}"
    );
    assert!(
        !code.contains("\nenum State") && !code.contains("\nenum Event"),
        "a file-level enumeration would be declared again by every procedure of the module: {code}"
    );
}

#[test]
fn a_final_state_is_named_as_the_document_wrote_it() {
    let code = swift_of("tests/forge/resources/procedure_startup_check.scxml")
        .expect("a procedure is written");
    assert!(
        code.contains("if state == .failVoltage { return \"fail_voltage\" }"),
        "the case is lowerCamel and the name is the document's id: {code}"
    );
}

#[test]
fn an_address_is_sent_as_decimal_text_or_as_written() {
    let code = swift_of("tests/forge/resources/procedure_address_forms.scxml")
        .expect("a procedure is written");
    assert!(
        code.contains("addr: String(ecuAddr)"),
        "an unsigned integer: {code}"
    );
    assert!(
        code.contains("addr: String(offset)"),
        "a signed integer: {code}"
    );
    assert!(
        code.contains("addr: gateway)"),
        "a string needs no conversion: {code}"
    );
}

#[test]
fn a_helper_is_a_closure_that_stops_until_the_host_sets_it() {
    let code = swift_of("tests/forge/resources/procedure_security_access.scxml")
        .expect("a procedure is written");
    assert!(
        code.contains("private var computeKey: ([UInt8]) -> [UInt8] = { _ in fatalError(\"helper 'computeKey' not set"),
        "{code}"
    );
    assert!(
        code.contains("public func setComputeKey(_ fn: @escaping ([UInt8]) -> [UInt8]) {"),
        "{code}"
    );
}

#[test]
fn a_bytes_value_past_the_cap_of_its_variable_raises_an_error_and_is_not_stored() {
    let code = swift_of("tests/forge/resources/procedure_security_access.scxml")
        .expect("a procedure is written");
    assert!(
        code.contains("if sceValue.count > 64 {\n                        return .errorExecution\n                    }\n                    seed = sceValue"),
        "{code}"
    );
}

#[test]
fn the_data_of_the_event_is_the_property_the_runtime_holds() {
    let code = swift_of("tests/forge/resources/procedure_security_access.scxml")
        .expect("a procedure is written");
    assert!(
        code.contains("let sceValue = Array(pendingEventData.utf8)"),
        "{code}"
    );
}

#[test]
fn a_codec_is_a_value_the_class_holds_and_asks_for_the_bytes_it_writes() {
    let code = swift_of("tests/forge/resources/crossfile_procedure_codec.scxml")
        .expect("a procedure that holds a codec is written");
    // A value, so the procedure may set its fields.
    assert!(
        code.contains("\n    private var frame = CodecSimpleFrame()\n"),
        "{code}"
    );
    // Its `encode` writes into a sink and throws; the class asks it once, and a refusal is no payload.
    assert!(
        code.contains("private func sceEncodeFrame() -> [UInt8]? {\n        encodeToBytes { try frame.encode(into: &$0) }\n    }"),
        "{code}"
    );
    assert!(code.contains("sceEncodeFrame()"), "{code}");
}

#[test]
fn a_field_of_a_held_codec_is_set_as_a_member_of_the_class_is() {
    let code = swift_of("tests/forge/resources/crossfile_procedure_codec_mutate.scxml")
        .expect("a procedure that sets a field of a codec is written");
    assert!(code.contains("frame.msgId = msgId"), "{code}");
}

#[test]
fn a_filter_is_a_property_of_the_class_built_with_it_and_called_by_its_alias() {
    let code = swift_of("tests/forge/resources/crossfile_procedure_filter.scxml")
        .expect("a procedure that holds a filter is written");
    assert!(
        code.contains("\n    private let smoother = FilterLowPass()\n"),
        "{code}"
    );
    assert!(code.contains("smoother.update(rawSample)"), "{code}");
}

#[test]
fn a_state_whose_case_swift_reserves_is_refused_by_name() {
    // The id `Default` is no reserved word, and its case `default` is one.
    let refusal = swift_of_procedure(
        INPUT,
        "Default",
        r#"<state id="Default"><transition target="done"/></state><final id="done"/>"#,
    )
    .expect_err("`default` is not a case");
    assert!(refusal.contains("a case name Swift reserves"), "{refusal}");
}

#[test]
fn a_state_that_is_a_reserved_word_is_refused_when_the_document_is_validated() {
    let refusal = swift_of_procedure(
        INPUT,
        "default",
        r#"<state id="default"><transition target="done"/></state><final id="done"/>"#,
    )
    .expect_err("`default` is not a name");
    assert!(refusal.contains("ReservedCodeIdentifier"), "{refusal}");
}

#[test]
fn two_events_that_are_one_case_are_refused_and_not_merged() {
    let refusal = swift_of_procedure(
        INPUT,
        "wait",
        r#"<state id="wait">
             <transition event="go_now" target="again"/>
             <transition event="goNow" target="done"/>
           </state>
           <state id="again"><transition event="go_now" target="done"/></state>
           <final id="done"/>"#,
    )
    .expect_err("two events are not one");
    assert!(
        refusal.contains("which are the one case `goNow`"),
        "{refusal}"
    );
}

#[test]
fn an_event_that_is_the_case_of_no_event_is_refused() {
    let refusal = swift_of_procedure(
        INPUT,
        "wait",
        r#"<state id="wait"><transition event="no_event" target="done"/></state><final id="done"/>"#,
    )
    .expect_err("an event may not be the case of no event");
    assert!(
        refusal.contains("which are the one case `noEvent`"),
        "{refusal}"
    );
}

#[test]
fn a_variable_named_for_a_member_of_the_class_is_refused() {
    let refusal = swift_of_procedure(
        r#"<data id="doneData" sce:type="int32" sce:direction="in"/>"#,
        "wait",
        r#"<state id="wait"><transition target="done"/></state><final id="done"/>"#,
    )
    .expect_err("a property is declared once");
    assert!(
        refusal.contains("the name of a member the class has"),
        "{refusal}"
    );
}
