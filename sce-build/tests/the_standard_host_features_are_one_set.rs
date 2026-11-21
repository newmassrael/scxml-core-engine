// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The standard host features (docs/SCE_HOST_FEATURES.md) are one set: a table
//! of invoke types ([`sce_build::forge::standard_host::TYPES`]), the records the
//! standard library holds for them, and what a build does with a document that
//! runs one.
//!
//! The first case ties the table to the library in both directions, because a
//! type with no record is a promise nothing keeps and a record no type names is
//! one nothing reads. The rest drive the CLI, since the aliases of a typed
//! invoke resolve through the imports the way a build reads them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
use sce_build::forge::standard_host::{document, lookup, BOUND_RELATIONS, TYPES};
use sce_build::generator::Language;
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

/// `sce-codegen <args…> <doc>` with JSON diagnostics: whether it succeeded, and
/// everything it wrote.
fn run(args: &[&str], doc: &str, siblings: &[(&str, &str)]) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// Every backend, by the identifier `-l` takes.
fn languages() -> Vec<&'static str> {
    emitting(ForgeKind::Statechart)
        .map(Language::canonical_name)
        .collect()
}

/// A `sce-static` statechart that asks `sce-host-kv-get` for `key`, a string
/// the machine holds with the bound `capacity`, importing the records `request`
/// and `result` name as the standard library's.
fn kv_get(capacity: &str, request: &str, result: &str, param: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="{request}" as="KvGetRequest"/>
  <sce:import kind="event-schema" src="{result}" as="KvGetResult"/>
  <datamodel>
    <data id="key" sce:type="string" sce:capacity="{capacity}" expr="''"/>
    <data id="wide" sce:type="int64" expr="0"/>
    <data id="found" sce:type="bool" expr="false" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="asking"/>
  </state>
  <state id="asking">
    <invoke type="sce-host-kv-get" id="get" sce:request="KvGetRequest" sce:result="KvGetResult">
      <param name="key" expr="{param}"/>
    </invoke>
    <transition event="done.invoke.get" target="idle">
      <assign location="found" expr="_event.data.found"/>
    </transition>
  </state>
</scxml>
"#
    )
}

const REQUEST: &str = "sce:std/host/kv_get_request.scxml";
const RESULT: &str = "sce:std/host/kv_get_result.scxml";

/// The file names under `stdlib/host`, without the extension.
fn library_documents() -> BTreeSet<String> {
    std::fs::read_dir(repo_root().join("stdlib/host"))
        .expect("stdlib/host")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .map(|path| {
            path.file_stem()
                .expect("a file stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// The enums the records `named` import, by their name under `stdlib/host`
/// without the extension: the documents beside the records that no type names
/// and a record of the table reads (docs/adr/0109, step 4 of its order of
/// work). Each is held to be an enum, since nothing else is a record's to
/// import.
fn imported_enums(named: &BTreeSet<String>) -> BTreeSet<String> {
    let mut enums = BTreeSet::new();
    for name in named {
        let path = repo_root().join(format!("stdlib/host/{name}.scxml"));
        let text = std::fs::read_to_string(&path).expect("a record of the library");
        for src in sce_build::forge::stdlib::import_sources(&text) {
            let stem = Path::new(&src)
                .file_stem()
                .expect("an imported document has a file name")
                .to_string_lossy()
                .into_owned();
            let imported =
                std::fs::read_to_string(repo_root().join(format!("stdlib/host/{stem}.scxml")))
                    .unwrap_or_else(|_| {
                        panic!("{name} imports {src}, which stdlib/host does not hold")
                    });
            assert!(
                imported.contains(r#"sce:kind="enum""#),
                "{name} imports {src}, which is no enum"
            );
            enums.insert(stem);
        }
    }
    enums
}

#[test]
fn the_table_and_the_library_name_each_other() {
    let named: BTreeSet<String> = TYPES
        .iter()
        .flat_map(|t| t.request.into_iter().chain(Some(t.result)))
        .map(str::to_string)
        .collect();
    let held = library_documents();
    assert!(
        held.len() >= 10,
        "stdlib/host held {} documents: a test that read none would pass",
        held.len()
    );
    let enums = imported_enums(&named);
    assert!(
        enums.contains("key_use"),
        "the enum key_generate_request imports was not read: {enums:?}"
    );
    let read: BTreeSet<String> = named.union(&enums).cloned().collect();
    assert_eq!(
        read, held,
        "a document of the library no type names and no record imports, or a type naming a \
         record the library lacks"
    );
    let embedded: BTreeSet<String> = sce_build::forge::stdlib::documents()
        .map(|(name, _)| name)
        .collect();
    for name in &read {
        assert!(
            embedded.contains(&document(name)),
            "{name} is not embedded in the generator"
        );
    }
    let types: BTreeSet<&str> = TYPES.iter().map(|t| t.invoke_type).collect();
    assert_eq!(types.len(), TYPES.len(), "two operations share one type");
    for t in TYPES {
        assert!(
            t.invoke_type.starts_with("sce-host-")
                && !sce_build::host_processor_analyzer::is_reserved_type(t.invoke_type),
            "{} is not a type a host may serve",
            t.invoke_type
        );
        assert_eq!(lookup(t.invoke_type), Some(t));
    }
}

#[test]
fn a_standard_type_is_served_without_a_declaration() {
    let doc = kv_get("256", REQUEST, RESULT, "key");
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &[]);
        assert!(ok, "{language} refused a standard host invoke: {text}");
        assert!(
            text.contains(r#""host_invoker_types":["sce-host-kv-get"]"#),
            "{language}: the type was not served: {text}"
        );
    }
}

#[test]
fn a_standard_type_is_held_to_the_records_of_the_library() {
    let own = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="kv_get_request" sce:kind="event-schema" sce:event-name="host.kv.get.request">
  <datamodel>
    <data id="key" sce:type="string" sce:max-size="256" sce:direction="in"/>
  </datamodel>
</scxml>"#;
    let doc = kv_get("256", "kv_get_request.scxml", RESULT, "key");
    let (ok, text) = run(&["check"], &doc, &[("kv_get_request.scxml", own)]);
    assert!(
        !ok,
        "a record of the same name and shape was accepted: {text}"
    );
    assert!(
        text.contains("validation/typed-invoke-schema") && text.contains("sce:std/host/"),
        "{text}"
    );
}

#[test]
fn a_standard_type_names_its_result() {
    let doc = kv_get("256", REQUEST, RESULT, "key").replace(r#" sce:result="KvGetResult""#, "");
    let (ok, text) = run(&["check"], &doc, &[]);
    assert!(
        !ok,
        "a standard invoke with no result record was accepted: {text}"
    );
    assert!(text.contains("validation/typed-invoke-schema"), "{text}");
}

#[test]
fn an_operation_with_no_request_takes_no_argument() {
    let doc = |param: &str| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="asking" datamodel="ecmascript" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/clock_now_result.scxml" as="Now"/>
  <state id="asking">
    <invoke type="sce-host-clock-now" id="now" sce:result="Now">{param}</invoke>
  </state>
</scxml>
"#
        )
    };
    let (ok, text) = run(&["check"], &doc(""), &[]);
    assert!(ok, "a clock read with no argument was refused: {text}");
    let (ok, text) = run(&["check"], &doc(r#"<param name="zone" expr="'x'"/>"#), &[]);
    assert!(!ok, "an argument to an operation that takes none: {text}");
    assert!(text.contains("validation/typed-invoke-request"), "{text}");
    // The engine's own deadline is not an argument.
    let (ok, text) = run(
        &["check"],
        &doc(r#"<param name="_sce_deadline_ms" expr="500"/>"#),
        &[],
    );
    assert!(ok, "a deadline was counted as an argument: {text}");
}

#[test]
fn a_value_the_machine_computes_is_held_to_the_field_it_supplies() {
    // A string held by a variable whose bound is the field's own is shown to
    // fit, on every backend.
    let doc = kv_get("256", REQUEST, RESULT, "key");
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &[]);
        assert!(ok, "{language}: {text}");
    }
    // One bounded wider than the field might not fit.
    let (ok, text) = run(&["check"], &kv_get("512", REQUEST, RESULT, "key"), &[]);
    assert!(
        !ok,
        "a string of 512 bytes was taken for a field of 256: {text}"
    );
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
    // A value of the wrong type is no value of the field.
    let (ok, text) = run(&["check"], &kv_get("256", REQUEST, RESULT, "wide"), &[]);
    assert!(!ok, "an integer was taken for a string field: {text}");
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
    // A text no declaration bounds is a run-time fact.
    let (ok, text) = run(
        &["check"],
        &kv_get("256", REQUEST, RESULT, "key + key"),
        &[],
    );
    assert!(
        !ok,
        "a text of no stated bound was taken for a field: {text}"
    );
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
}

// ── Signing and sealing (docs/SCE_HOST_FEATURES.md, section 10) ──

/// A `sce-static` statechart that signs the bytes it holds with the key its
/// host names `k`, and keeps the signed object.
const SIGN: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/cose_sign1_request.scxml" as="Sign1Request"/>
  <sce:import kind="event-schema" src="sce:std/host/cose_sign1_result.scxml" as="Sign1Result"/>
  <datamodel>
    <data id="key_id" sce:type="string[64]" expr="'k'"/>
    <data id="payload" sce:type="bytes[4096]" expr="'ab'"/>
    <data id="aad" sce:type="bytes[256]" expr="''"/>
    <data id="signed" sce:type="bytes[4352]" expr="''" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="signing"/>
  </state>
  <state id="signing">
    <invoke type="sce-host-cose-sign1" id="sign" sce:request="Sign1Request" sce:result="Sign1Result">
      <param name="key_id" expr="key_id"/>
      <param name="payload" expr="payload"/>
      <param name="external_aad" expr="aad"/>
    </invoke>
    <transition event="done.invoke.sign" target="idle">
      <assign location="signed" expr="_event.data.cose"/>
    </transition>
  </state>
</scxml>
"#;

/// The signing and sealing operations are served on the lanes whose reference
/// host serves them, Rust and C++, and a build for any other lane refuses the
/// machine on its `<invoke>`, naming the lanes that serve it (docs/adr/0107,
/// D7 and D8).
#[test]
fn signing_is_served_on_rust_and_cpp_and_refused_on_every_other_lane() {
    let lanes = languages();
    assert!(
        lanes.contains(&"rust") && lanes.contains(&"cpp"),
        "the lanes that serve signing are not among the emitting lanes: {lanes:?}"
    );
    // Every emitting lane but the two that serve the operations is refused.
    // Counted from the emitting set rather than written down, so a lane that
    // starts emitting statecharts is held here the day it does.
    let expected = lanes
        .iter()
        .filter(|language| !matches!(**language, "rust" | "cpp"))
        .count();
    let mut refused_on = 0;
    for language in lanes {
        let (ok, text) = run(&["check", "-l", language], SIGN, &[]);
        if matches!(language, "rust" | "cpp") {
            assert!(ok, "{language} refused a machine that signs: {text}");
            assert!(
                text.contains(r#""host_invoker_types":["sce-host-cose-sign1"]"#),
                "{language}: the type was not served: {text}"
            );
        } else {
            assert!(
                !ok,
                "{language} took a machine its host cannot sign for: {text}"
            );
            assert!(
                text.contains("codegen/standard-host-type-not-served")
                    && text.contains("sce-host-cose-sign1")
                    && text.contains("rust, cpp"),
                "{language}: {text}"
            );
            refused_on += 1;
        }
    }
    assert!(
        expected > 0 && refused_on == expected,
        "refused on {refused_on} of the {expected} emitting lanes that do not serve signing"
    );
}

// ── Random bytes, keyed digests and keys (docs/SCE_HOST_FEATURES.md, sections 11 to 13) ──

/// A `sce-static` statechart that runs `invoke_type` with the library's records
/// `request` and `result`, importing besides them the library's enums
/// `enums`, `(document, alias)`, each request field given from a variable of
/// the field's own type: `(field, type, initial value)`.
fn host_operation(
    invoke_type: &str,
    request: &str,
    result: &str,
    enums: &[(&str, &str)],
    fields: &[(&str, &str, &str)],
) -> String {
    let imports: String = enums
        .iter()
        .map(|(document, alias)| {
            format!(
                "  <sce:import kind=\"enum\" src=\"sce:std/host/{document}.scxml\" as=\"{alias}\"/>\n"
            )
        })
        .collect();
    let variables: String = fields
        .iter()
        .map(|(name, ty, init)| {
            format!("    <data id=\"{name}\" sce:type=\"{ty}\" expr=\"{init}\"/>\n")
        })
        .collect();
    let params: String = fields
        .iter()
        .map(|(name, _, _)| format!("      <param name=\"{name}\" expr=\"{name}\"/>\n"))
        .collect();
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/{request}.scxml" as="Request"/>
  <sce:import kind="event-schema" src="sce:std/host/{result}.scxml" as="Result"/>
{imports}  <datamodel>
{variables}    <data id="failed" sce:type="string[24]" expr="''" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="asking"/>
  </state>
  <state id="asking">
    <invoke type="{invoke_type}" id="op" sce:request="Request" sce:result="Result">
{params}    </invoke>
    <transition event="done.invoke.op" target="idle">
      <assign location="failed" expr="_event.data.error"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// The library's enum of a key's use, under the alias a machine imports it by.
const KEY_USE_ENUM: (&str, &str) = ("key_use", "KeyUse");

/// A key's use as a variable of that enum, the field's own type.
const KEY_USE_MAC: (&str, &str, &str) = ("key_use", "enum:KeyUse", "KeyUse.mac");

/// docs/adr/0109, D0: random bytes, keyed digests and keys are built for Rust
/// and C++, and a build for every other emitting lane refuses a machine that
/// asks for one by name, `codegen/standard-host-type-not-served`, naming the
/// two lanes that serve it; never a machine that silently asks a host that
/// cannot answer.
#[test]
fn random_bytes_keyed_digests_and_keys_are_served_on_rust_and_cpp_and_refused_elsewhere() {
    let key_id = ("key_id", "string[64]", "'k'");
    let message = ("message", "bytes[mac_message=4096]", "'m'");
    let none: Vec<(&str, &str)> = Vec::new();
    let operations = [
        (
            "sce-host-random-bytes",
            "random_bytes_request",
            "random_bytes_result",
            none.clone(),
            vec![("length", "uint16", "16")],
        ),
        (
            "sce-host-mac",
            "mac_request",
            "mac_result",
            none.clone(),
            vec![key_id, message],
        ),
        (
            "sce-host-mac-verify",
            "mac_verify_request",
            "mac_verify_result",
            none.clone(),
            vec![key_id, message, ("tag", "bytes[32]", "''")],
        ),
        (
            "sce-host-key-generate",
            "key_generate_request",
            "key_generate_result",
            vec![KEY_USE_ENUM],
            vec![KEY_USE_MAC, ("label", "string[64]", "''")],
        ),
        (
            "sce-host-key-public",
            "key_public_request",
            "key_public_result",
            none.clone(),
            vec![key_id],
        ),
        (
            "sce-host-key-retire",
            "key_retire_request",
            "key_retire_result",
            none,
            vec![key_id],
        ),
    ];
    held_to_rust_and_cpp(&operations);
}

/// docs/adr/0109, D0 and D3: issuing and revoking credentials are built for
/// Rust and C++, and a build for every other emitting lane refuses a machine
/// that asks for either by name, `codegen/standard-host-type-not-served`,
/// naming the two lanes that serve it.
#[test]
fn credentials_are_served_on_rust_and_cpp_and_refused_elsewhere() {
    let name = ("name", "string[256]", "'phone'");
    let none: Vec<(&str, &str)> = Vec::new();
    let operations = [
        (
            "sce-host-credential-issue",
            "credential_issue_request",
            "credential_issue_result",
            none.clone(),
            vec![name, ("ttl_s", "uint32", "3600")],
        ),
        (
            "sce-host-credential-revoke",
            "credential_revoke_request",
            "credential_revoke_result",
            none,
            vec![name],
        ),
    ];
    held_to_rust_and_cpp(&operations);
}

/// An operation a probe runs: its `type`, its request and result records, the
/// library's enums it imports, `(document, alias)`, and each request field with
/// its type and the value the probe gives it.
type ProbedOperation<'a> = (
    &'a str,
    &'a str,
    &'a str,
    Vec<(&'a str, &'a str)>,
    Vec<(&'a str, &'a str, &'a str)>,
);

/// Each of `operations`, run by a probe of [`host_operation`], is served on
/// Rust and C++ and refused by name on every other emitting lane, naming the
/// two that serve it (docs/adr/0109, D0).
fn held_to_rust_and_cpp(operations: &[ProbedOperation<'_>]) {
    let lanes = languages();
    let others = lanes
        .iter()
        .filter(|language| !matches!(**language, "rust" | "cpp"))
        .count();
    assert!(
        others > 0,
        "no emitting lane but rust and cpp: the refusal is held on none"
    );
    for (invoke_type, request, result, enums, fields) in operations {
        let doc = host_operation(invoke_type, request, result, enums, fields);
        let mut refused_on = 0;
        for &language in &lanes {
            let (ok, text) = run(&["check", "-l", language], &doc, &[]);
            if matches!(language, "rust" | "cpp") {
                assert!(ok, "{language} refused {invoke_type}: {text}");
                assert!(
                    text.contains(&format!(r#""host_invoker_types":["{invoke_type}"]"#)),
                    "{language}: {invoke_type} was not served: {text}"
                );
            } else {
                assert!(
                    !ok,
                    "{language} took a machine its host cannot answer {invoke_type} for: {text}"
                );
                assert!(
                    text.contains("codegen/standard-host-type-not-served")
                        && text.contains(*invoke_type)
                        && text.contains("rust, cpp"),
                    "{language}: {text}"
                );
                refused_on += 1;
            }
        }
        assert_eq!(
            refused_on, others,
            "{invoke_type} was refused on {refused_on} of the {others} lanes that do not serve it"
        );
    }
}

/// docs/adr/0109, step 4 of its order of work: a key's use is a value of the
/// library's enum `key_use`, so a use that is none of its four is refused where
/// it is written rather than judged by the host when the invocation starts: its
/// name as a text, a text a variable holds, and a text where the machine does
/// not import the enum. The field's own value is accepted on the lanes that
/// serve the operation.
#[test]
fn a_key_use_that_is_no_value_of_its_enum_is_refused_where_it_is_written() {
    let key_generation = |enums: &[(&str, &str)], key_use: (&str, &str, &str)| {
        host_operation(
            "sce-host-key-generate",
            "key_generate_request",
            "key_generate_result",
            enums,
            &[key_use, ("label", "string[64]", "''")],
        )
    };
    let accepted = key_generation(&[KEY_USE_ENUM], KEY_USE_MAC);
    for language in ["rust", "cpp"] {
        let (ok, text) = run(&["check", "-l", language], &accepted, &[]);
        assert!(ok, "{language} refused a use of the enum: {text}");
    }

    let named = accepted.replace(
        r#"<param name="key_use" expr="key_use"/>"#,
        r#"<param name="key_use" expr="'mac'"/>"#,
    );
    assert_ne!(named, accepted, "the probe's <param> was not rewritten");
    let (ok, text) = run(&["check"], &named, &[]);
    assert!(!ok, "the name of a use was taken as the use: {text}");
    assert!(text.contains("validation/typed-invoke-request"), "{text}");

    for (enums, why) in [
        (&[KEY_USE_ENUM][..], "no value of an enum"),
        (&[][..], "does not import"),
    ] {
        let doc = key_generation(enums, ("key_use", "string[8]", "'mac'"));
        let (ok, text) = run(&["check"], &doc, &[]);
        assert!(!ok, "a text was taken as a key's use: {text}");
        assert!(
            text.contains("scxml/static-datamodel-rule") && text.contains(why),
            "{why}: {text}"
        );
    }
}

// ── A model's generation and its answer schema (docs/adr/0087, section 4) ──

/// The answer schema the probes below name: a title and a slot.
const ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="answer" sce:event-name="probe.answer">
  <datamodel>
    <data id="title" sce:type="string[16]" sce:direction="in"/>
    <data id="slot" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// An open enum, which has no grammar: a value of it may be a number no name
/// spells.
const OPEN_KIND: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="open_kind" sce:underlying-type="uint8"
       sce:strict-variants="false">
  <datamodel>
    <data id="variants">
      <sce:variant name="meeting" value="0"/>
    </data>
  </datamodel>
</scxml>"#;

/// An answer schema holding the open enum.
const OPEN_ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="open_answer" sce:event-name="probe.open">
  <sce:import kind="enum" src="open_kind.scxml" as="OpenKind"/>
  <datamodel>
    <data id="kind" sce:type="enum:OpenKind" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// An answer schema whose record holds a list of records with a lower count:
/// a plan of one to three steps (docs/adr/0087, "Landing order" item 8).
const PLAN: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="plan" sce:event-name="probe.plan">
  <sce:import kind="event-schema" src="plan_step.scxml" as="Step"/>
  <datamodel>
    <data id="steps" sce:type="list&lt;record:Step&gt;[1..3]" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// One step of [`PLAN`].
const PLAN_STEP: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="plan_step" sce:event-name="probe.plan.step">
  <datamodel>
    <data id="title" sce:type="string[8]" sce:direction="in"/>
    <data id="at" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// The siblings every model probe is read beside.
const MODEL_SIBLINGS: [(&str, &str); 5] = [
    ("answer.scxml", ANSWER),
    ("open_kind.scxml", OPEN_KIND),
    ("open_answer.scxml", OPEN_ANSWER),
    ("plan.scxml", PLAN),
    ("plan_step.scxml", PLAN_STEP),
];

/// A `sce-static` statechart that asks `invoke_type` for a text, the invoke
/// carrying `attrs` (an `sce:answer`, say) and the extra `<param>`s `extra`.
fn model_generate(invoke_type: &str, attrs: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/model_generate_request.scxml" as="GenerateRequest"/>
  <sce:import kind="event-schema" src="sce:std/host/model_generate_result.scxml" as="GenerateResult"/>
  <sce:import kind="event-schema" src="answer.scxml" as="Answer"/>
  <datamodel>
    <data id="prompt" sce:type="string[64]" expr="'Name a day'"/>
    <data id="said" sce:type="string[4096]" expr="''" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="asking"/>
  </state>
  <state id="asking">
    <invoke type="{invoke_type}" id="gen"{attrs} sce:request="GenerateRequest" sce:result="GenerateResult">
      <param name="prompt" expr="prompt"/>
      <param name="max_tokens" expr="64"/>
      <param name="temperature_milli" expr="0"/>{extra}
    </invoke>
    <transition event="done.invoke.gen" target="idle">
      <assign location="said" expr="_event.data.text"/>
    </transition>
  </state>
</scxml>
"#
    )
}

const MODEL_GENERATE: &str = "sce-host-model-generate";

/// The id of `answer.scxml`'s payload schema, which the build writes into the
/// request.
const ANSWER_ID: &str = "urn:sce:schema:answer:1";

/// `sce-codegen generate -l <language>` of `doc` beside the model siblings:
/// whether it succeeded, what it said, and the text of every file of the
/// machine it wrote (named after the machine, `probe`), so that a schema unit
/// written beside it, which declares its own id and JSON Schema, is not read as
/// the machine's.
fn generate(language: &str, doc: &str) -> (bool, String, String) {
    generate_beside(language, doc, &MODEL_SIBLINGS)
}

/// [`generate`] of `doc` beside `siblings`.
fn generate_beside(language: &str, doc: &str, siblings: &[(&str, &str)]) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", language]);
    if language == "go" {
        command.args(["--go-module-prefix", "example.test/generated"]);
    }
    let out = command
        .arg(&path)
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    let mut written = String::new();
    let mut machine_files = 0;
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        let of_machine = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_lowercase().contains("probe"));
        if path.is_file() && of_machine {
            machine_files += 1;
            written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            written.push('\n');
        }
    }
    assert!(
        !out.status.success() || machine_files > 0,
        "{language} wrote no file named after the machine: a test that read none would pass"
    );
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written,
    )
}

#[test]
fn a_model_generation_is_served_on_every_backend_with_its_answer_schema() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Answer""#, "");
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &MODEL_SIBLINGS);
        assert!(ok, "{language} refused a model's generation: {text}");
        assert!(
            text.contains(&format!(r#""host_invoker_types":["{MODEL_GENERATE}"]"#)),
            "{language}: the type was not served: {text}"
        );
    }
}

/// The build writes the answer schema's id into the request, and declares the
/// machine's table of answer schemas, the id beside both of its grammars. C and
/// C++ hold each grammar as characters, so only the id is read as text there.
#[test]
fn the_build_writes_the_answer_schema_and_the_table_of_its_grammars() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Answer""#, "");
    for language in languages() {
        let (ok, text, written) = generate(language, &doc);
        assert!(ok, "{language} did not generate: {text}");
        assert!(
            written.contains(ANSWER_ID),
            "{language}: neither the request nor the table names {ANSWER_ID}:\n{written}"
        );
        if !matches!(language, "c11" | "cpp") {
            assert!(
                written.contains("root ::="),
                "{language}: the table holds no GBNF grammar:\n{written}"
            );
            assert!(
                written.contains("json-schema.org/draft/2020-12/schema"),
                "{language}: the table holds no JSON Schema:\n{written}"
            );
        }
    }
}

/// An answer schema whose record holds a list of records is named as any other
/// is (docs/adr/0087, "Landing order" item 8): the build writes its id into the
/// request, and the machine's table holds the composed record's JSON Schema,
/// its list's lower and upper counts and the step's record among its `$defs`,
/// beside its GBNF grammar, both folded from the type as a record of entries'
/// are. C and C++ hold each grammar as characters, so only the id is read as
/// text there.
#[test]
fn an_answer_schema_whose_record_holds_a_list_is_in_the_table_with_its_grammars() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Plan""#, "").replace(
        r#"<sce:import kind="event-schema" src="answer.scxml" as="Answer"/>"#,
        r#"<sce:import kind="event-schema" src="plan.scxml" as="Plan"/>
  <sce:import kind="event-schema" src="plan_step.scxml" as="Step"/>"#,
    );
    assert!(doc.contains(r#"as="Plan""#), "the probe imports the plan");
    for language in languages() {
        let (ok, text, written) = generate(language, &doc);
        assert!(ok, "{language} did not generate: {text}");
        assert!(
            written.contains("urn:sce:schema:plan:1"),
            "{language}: neither the request nor the table names the plan:\n{written}"
        );
        if !matches!(language, "c11" | "cpp") {
            // `defs/plan_step` is the step's `$ref`, read past the `$` a
            // language escapes in a text.
            for part in ["minItems", "maxItems", "defs/plan_step", "root ::="] {
                assert!(
                    written.contains(part),
                    "{language}: the table holds no `{part}`:\n{written}"
                );
            }
        }
    }
}

/// An invoke that names no answer schema asks for a free text: its request's
/// `answer_schema` is empty and the machine declares no table.
#[test]
fn a_generation_that_names_no_answer_schema_declares_no_table() {
    let doc = model_generate(MODEL_GENERATE, "", "");
    let (ok, text, written) = generate("rust", &doc);
    assert!(ok, "a free generation was refused: {text}");
    assert!(
        !written.contains("ANSWER_SCHEMAS") && !written.contains("urn:sce:schema:"),
        "a machine that names no answer schema declared one:\n{written}"
    );
}

/// A literal written into an unsigned field of a typed request takes the field's
/// type (the probes write `max_tokens` as `64` into a `uint32`), and one the
/// field's range does not hold is refused where it is written, never narrowed.
#[test]
fn a_literal_takes_the_type_of_the_request_field_it_supplies() {
    let accepted = model_generate(MODEL_GENERATE, "", "");
    let (ok, text) = run(&["check"], &accepted, &MODEL_SIBLINGS);
    assert!(ok, "a literal an unsigned field holds was refused: {text}");
    for (field, past) in [("max_tokens", "-1"), ("temperature_milli", "70000")] {
        let doc = accepted.replace(
            &format!(
                "<param name=\"{field}\" expr=\"{}\"/>",
                if field == "max_tokens" { "64" } else { "0" }
            ),
            &format!("<param name=\"{field}\" expr=\"{past}\"/>"),
        );
        assert_ne!(
            doc, accepted,
            "the probe writes {field} as the test expects"
        );
        let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
        assert!(!ok, "{past} was taken for {field}: {text}");
        assert!(
            text.contains("expression/literal-out-of-range"),
            "{field} = {past}: {text}"
        );
    }
}

/// The document does not write the field the build writes.
#[test]
fn a_document_does_not_write_the_answer_schema_of_a_request() {
    let doc = model_generate(
        MODEL_GENERATE,
        r#" sce:answer="Answer""#,
        "\n      <param name=\"answer_schema\" expr=\"'urn:sce:schema:other:1'\"/>",
    );
    let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
    assert!(!ok, "a document wrote the answer schema itself: {text}");
    assert!(
        text.contains("validation/typed-invoke-request") && text.contains("sce:answer"),
        "{text}"
    );
}

#[test]
fn an_answer_schema_is_refused_on_another_type() {
    let doc = kv_get("256", REQUEST, RESULT, "key").replace(
        r#"<invoke type="sce-host-kv-get" id="get""#,
        r#"<invoke type="sce-host-kv-get" id="get" sce:answer="KvGetResult""#,
    );
    let (ok, text) = run(&["check"], &doc, &[]);
    assert!(!ok, "an answer schema on a key read was accepted: {text}");
    assert!(
        text.contains("validation/model-answer-schema") && text.contains(MODEL_GENERATE),
        "{text}"
    );
}

#[test]
fn an_answer_schema_the_document_does_not_import_is_refused() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Answr""#, "");
    let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
    assert!(!ok, "an alias no import names was accepted: {text}");
    assert!(
        text.contains("validation/model-answer-schema") && text.contains("Answr"),
        "{text}"
    );
}

#[test]
fn an_answer_schema_with_no_grammar_is_refused() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="OpenAnswer""#, "").replace(
        r#"<sce:import kind="event-schema" src="answer.scxml" as="Answer"/>"#,
        r#"<sce:import kind="event-schema" src="open_answer.scxml" as="OpenAnswer"/>"#,
    );
    let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
    assert!(
        !ok,
        "an answer schema holding an open enum was accepted: {text}"
    );
    assert!(
        text.contains("validation/model-answer-schema") && text.contains("grammar"),
        "{text}"
    );
}

// ── The audit log (docs/SCE_HOST_FEATURES.md, section 15; docs/adr/0109, D14) ──

/// The entry schema an audit probe names: an account marked personal and a
/// password marked secret, beside a field kept as it is.
const SIGN_IN: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="sign_in" sce:event-name="sign.in">
  <datamodel>
    <data id="account" sce:type="string[64]" sce:direction="in" sce:sensitivity="personal"/>
    <data id="password" sce:type="string[32]" sce:direction="in" sce:sensitivity="secret"/>
    <data id="result" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// The marks the build writes for [`SIGN_IN`].
const SIGN_IN_MARKS: &str = "/account=personal;/password=secret";

/// A `sce-static` statechart that appends the record it holds to a stream,
/// the invoke carrying `attrs` (its `sce:entry`) and the entry given as
/// `entry`, with the extra `<param>`s `extra`.
fn audit_append(attrs: &str, entry: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/audit_append_request.scxml" as="AppendRequest"/>
  <sce:import kind="event-schema" src="sce:std/host/audit_append_result.scxml" as="AppendResult"/>
  <sce:import kind="event-schema" src="sign_in.scxml" as="SignIn"/>
  <datamodel>
    <data id="attempt" sce:type="record:SignIn">
      <sce:set name="account" expr="'alice'"/>
      <sce:set name="password" expr="'hunter2'"/>
      <sce:set name="result" expr="'accepted'"/>
    </data>
    <data id="note" sce:type="string[64]" expr="''"/>
    <data id="sequence" sce:type="string[20]" expr="''" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="appending"/>
  </state>
  <state id="appending">
    <invoke type="sce-host-audit-append" id="log"{attrs} sce:request="AppendRequest" sce:result="AppendResult">
      <param name="stream" expr="'sign-ins'"/>
      <param name="entry" expr="{entry}"/>{extra}
    </invoke>
    <transition event="done.invoke.log" target="idle">
      <assign location="sequence" expr="_event.data.sequence"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// docs/adr/0109, D0 and D14: the audit log is built for Rust and C++, and a
/// build for every other emitting lane refuses a machine that appends to it by
/// name, naming the two lanes that serve it; on the two, the build writes the
/// marks of the entry's schema into the request.
#[test]
fn an_audit_append_is_served_on_rust_and_cpp_and_refused_elsewhere() {
    let doc = audit_append(r#" sce:entry="SignIn""#, "attempt", "");
    let siblings = [("sign_in.scxml", SIGN_IN)];
    let lanes = languages();
    for &language in &lanes {
        let (ok, text) = run(&["check", "-l", language], &doc, &siblings);
        if matches!(language, "rust" | "cpp") {
            assert!(ok, "{language} refused an audit append: {text}");
            assert!(
                text.contains(r#""host_invoker_types":["sce-host-audit-append"]"#),
                "{language}: the append was not served: {text}"
            );
        } else {
            assert!(!ok, "{language} took an audit append: {text}");
            assert!(
                text.contains("codegen/standard-host-type-not-served")
                    && text.contains("sce-host-audit-append")
                    && text.contains("rust, cpp"),
                "{language}: {text}"
            );
        }
    }
}

/// The build writes the marks of the entry's schema into the request's
/// `sensitivity` on the lanes that serve the append, where the generated
/// machine holds them as a literal.
#[test]
fn the_build_writes_the_marks_of_the_entry_schema() {
    let doc = audit_append(r#" sce:entry="SignIn""#, "attempt", "");
    for language in ["rust", "cpp"] {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join("sign_in.scxml"), SIGN_IN).expect("write sibling");
        let path = dir.path().join("probe.scxml");
        std::fs::write(&path, &doc).expect("write probe");
        let out_dir = tempdir().expect("tempdir");
        let out = Command::new(sce_codegen_bin())
            .arg("--workspace-root")
            .arg(repo_root())
            .arg("--error-format=json")
            .args(["generate", "-l", language])
            .arg(&path)
            .arg("-o")
            .arg(out_dir.path())
            .output()
            .expect("invoke sce-codegen");
        assert!(
            out.status.success(),
            "{language} did not generate: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut written = String::new();
        for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_file() {
                written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
        assert!(
            written.contains(SIGN_IN_MARKS),
            "{language}: the request carries no marks {SIGN_IN_MARKS}"
        );
    }
}

/// The document does not write the field the build writes.
#[test]
fn a_document_does_not_write_the_marks_of_an_audit_entry() {
    let doc = audit_append(
        r#" sce:entry="SignIn""#,
        "attempt",
        "\n      <param name=\"sensitivity\" expr=\"''\"/>",
    );
    let (ok, text) = run(&["check"], &doc, &[("sign_in.scxml", SIGN_IN)]);
    assert!(!ok, "a document wrote the marks itself: {text}");
    assert!(
        text.contains("validation/typed-invoke-request") && text.contains("sce:entry"),
        "{text}"
    );
}

/// An append names its entry's schema, on an append only, and the schema is an
/// import; each refusal is `validation/audit-entry-schema`.
#[test]
fn an_entry_schema_is_named_on_an_append_and_imported() {
    let siblings = [("sign_in.scxml", SIGN_IN)];
    for (doc, why) in [
        (audit_append("", "attempt", ""), "names none"),
        (
            audit_append(r#" sce:entry="SignOn""#, "attempt", ""),
            "names no event schema this document imports",
        ),
        (
            kv_get("256", REQUEST, RESULT, "key").replace(
                r#"<invoke type="sce-host-kv-get" id="get""#,
                r#"<invoke type="sce-host-kv-get" id="get" sce:entry="KvGetResult""#,
            ),
            "sce-host-audit-append",
        ),
    ] {
        let (ok, text) = run(&["check"], &doc, &siblings);
        assert!(
            !ok,
            "an entry schema the build cannot write was accepted: {text}"
        );
        assert!(
            text.contains("validation/audit-entry-schema") && text.contains(why),
            "{text}"
        );
    }
}

/// The entry is a value of the record its `sce:entry` names, and nothing else:
/// a text is refused where it is written.
#[test]
fn an_entry_that_is_no_record_of_its_schema_is_refused() {
    let doc = audit_append(r#" sce:entry="SignIn""#, "note", "");
    let (ok, text) = run(
        &["check", "-l", "rust"],
        &doc,
        &[("sign_in.scxml", SIGN_IN)],
    );
    assert!(!ok, "a text was taken for an entry: {text}");
    assert!(
        text.contains("scxml/static-datamodel-rule") && text.contains("record:SignIn"),
        "{text}"
    );
}

/// A mark that is none of the two, and `personal` on a field that holds no
/// text, are refused where the schema is read.
#[test]
fn a_mark_is_secret_or_personal_and_personal_marks_a_text() {
    for (schema, code) in [
        (
            SIGN_IN.replace(r#"sce:sensitivity="secret""#, r#"sce:sensitivity="hidden""#),
            "validation/invalid-attribute",
        ),
        (
            SIGN_IN.replace(
                r#"<data id="result" sce:type="string[16]" sce:direction="in"/>"#,
                r#"<data id="result" sce:type="uint8" sce:direction="in" sce:sensitivity="personal"/>"#,
            ),
            "validation/attribute-rule-violated",
        ),
    ] {
        let doc = audit_append(r#" sce:entry="SignIn""#, "attempt", "");
        let (ok, text) = run(&["check"], &doc, &[("sign_in.scxml", schema.as_str())]);
        assert!(!ok, "a mark the build cannot carry was accepted: {text}");
        assert!(text.contains(code) && text.contains("sce:sensitivity"), "{text}");
    }
}

// ── The named bounds of signing and sealing (docs/adr/0109, D17) ──

/// The number each named bound of signing and sealing is written at in the
/// library: the bound every record of the operations held before the bounds
/// were named, which a build given no limit keeps.
const TODAYS_BOUNDS: [(&str, u32); 4] = [
    ("cose_payload", 4096),
    ("cose_signed", 4352),
    ("cose_plaintext", 4352),
    ("cose_sealed", 4480),
];

/// The number [`TODAYS_BOUNDS`] gives `name`.
fn todays_bound(name: &str) -> u32 {
    TODAYS_BOUNDS
        .iter()
        .find(|(bound, _)| *bound == name)
        .map(|(_, number)| *number)
        .unwrap_or_else(|| panic!("`{name}` is no named bound of signing or sealing"))
}

/// The text of the library's record `record`.
fn library_record(record: &str) -> String {
    std::fs::read_to_string(repo_root().join(format!("stdlib/host/{record}.scxml")))
        .unwrap_or_else(|_| panic!("{record} is no record of the library"))
}

/// A probe of the signing or sealing operation `invoke_type` (one of
/// [`host_operation`]'s), whose variable of the value it hands the host names
/// its record's bound, so a build that moves the bound moves the variable too.
fn cose_operation(invoke_type: &str) -> String {
    let standard = lookup(invoke_type).expect("an operation of the table");
    let (named, value, bound) = match invoke_type {
        "sce-host-cose-sign1" => ("key_id", "payload", "cose_payload"),
        "sce-host-cose-verify1" => ("trust_set", "cose", "cose_signed"),
        "sce-host-cose-encrypt0" => ("key_id", "plaintext", "cose_plaintext"),
        "sce-host-cose-decrypt0" => ("key_id", "cose", "cose_sealed"),
        other => panic!("{other} neither signs nor seals"),
    };
    let ty = format!("bytes[{bound}={}]", todays_bound(bound));
    host_operation(
        invoke_type,
        standard
            .request
            .expect("signing and sealing take a request"),
        standard.result,
        &[],
        &[
            (named, "string[64]", "'k'"),
            (value, ty.as_str(), "''"),
            ("external_aad", "bytes[256]", "''"),
        ],
    )
}

/// docs/adr/0109, D17: a build that moves the bound of what is signed or
/// sealed and not the bound of the object made of it is refused on the
/// `<invoke>`, by name, with the `--limit` that repairs it; the same build
/// with the object's bound moved too is taken, and so is one that moves the
/// content's bound down alone. Each operation that reads both bounds is held,
/// on the two lanes that serve it.
#[test]
fn a_build_that_breaks_a_relation_of_the_bounds_is_refused_by_name() {
    let cases = [
        (
            "sce-host-cose-sign1",
            "cose_payload",
            "cose_signed",
            8192,
            8448,
        ),
        (
            "sce-host-cose-verify1",
            "cose_payload",
            "cose_signed",
            8192,
            8448,
        ),
        (
            "sce-host-cose-encrypt0",
            "cose_plaintext",
            "cose_sealed",
            8448,
            8576,
        ),
        (
            "sce-host-cose-decrypt0",
            "cose_plaintext",
            "cose_sealed",
            8448,
            8576,
        ),
    ];
    for (operation, content, object, moved, least) in cases {
        let doc = cose_operation(operation);
        let (content_moved, object_moved, content_down) = (
            format!("{content}={moved}"),
            format!("{object}={least}"),
            format!("{content}=1024"),
        );
        let (content_moved, object_moved, content_down) = (
            content_moved.as_str(),
            object_moved.as_str(),
            content_down.as_str(),
        );
        for language in ["rust", "cpp"] {
            let (ok, text) = run(&["check", "-l", language], &doc, &[]);
            assert!(ok, "{language} {operation} built as written: {text}");

            let (ok, text) = run(
                &["--limit", content_moved, "check", "-l", language],
                &doc,
                &[],
            );
            assert!(
                !ok,
                "{language} {operation} took {content_moved} alone: {text}"
            );
            assert!(
                text.contains("codegen/standard-host-bound-relation")
                    && text.contains(operation)
                    && text.contains(&format!("--limit {object_moved} or more")),
                "{language} {operation}: {text}"
            );

            let (ok, text) = run(
                &[
                    "--limit",
                    content_moved,
                    "--limit",
                    object_moved,
                    "check",
                    "-l",
                    language,
                ],
                &doc,
                &[],
            );
            assert!(ok, "{language} {operation} refused both moved: {text}");

            let (ok, text) = run(
                &["--limit", content_down, "check", "-l", language],
                &doc,
                &[],
            );
            assert!(ok, "{language} {operation} refused {content_down}: {text}");
        }
    }
}

/// docs/adr/0109, D17: each relation's two bounds are written by the records
/// of each operation it names, one in the request and one in the result, at
/// the numbers those records wrote before the bounds were named, and the
/// relation holds at those numbers, so a build given no limit is refused
/// nothing it was given before.
#[test]
fn each_relation_is_written_by_the_records_of_its_operations_at_todays_numbers() {
    let mut read = 0;
    for relation in BOUND_RELATIONS {
        assert!(
            relation.holds(
                todays_bound(relation.content),
                todays_bound(relation.object)
            ),
            "{relation:?} does not hold at the library's numbers"
        );
        for &operation in relation.operations {
            let standard = lookup(operation).expect("an operation of the table");
            let request = library_record(standard.request.expect("a request"));
            let result = library_record(standard.result);
            for name in [relation.content, relation.object] {
                let spelled = format!("bytes[{name}={}]", todays_bound(name));
                assert_eq!(
                    usize::from(request.contains(&spelled))
                        + usize::from(result.contains(&spelled)),
                    1,
                    "{operation}: one of its records writes {spelled}"
                );
                read += 1;
            }
        }
    }
    assert_eq!(read, 8, "two bounds of four operations");
}

/// `sce-codegen [--limit L]... generate -l rust <document>`, `document` a path
/// or a library name: the lines of every file written that declare a field's
/// bound or the longest payload of the event, sorted.
fn bound_constants(document: &str, limits: &[&str]) -> Vec<String> {
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json");
    for limit in limits {
        command.args(["--limit", limit]);
    }
    let out = command
        .args(["generate", "-l", "rust", document, "-o"])
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "{document}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut lines: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let text = std::fs::read_to_string(entry.expect("an entry").path()).unwrap_or_default();
        lines.extend(
            text.lines()
                .filter(|line| {
                    line.starts_with("pub const")
                        && (line.contains("_MAX_SIZE") || line.contains("DATA_MAX_LEN"))
                })
                .map(str::to_string),
        );
    }
    lines.sort();
    assert!(
        lines.iter().any(|line| line.contains("DATA_MAX_LEN")),
        "{document} declared no longest payload: a test that read none would pass"
    );
    lines
}

/// docs/adr/0109, D17: naming the bounds moves no byte of a build given no
/// limit. Each record of signing and sealing, generated as the library writes
/// it, declares every field's bound and its longest payload as the same record
/// with its bounds written as bare numbers does; a build that moves a bound
/// moves the constant of the field that names it, and only that one.
#[test]
fn a_record_built_with_no_limit_declares_what_its_bare_numbers_declared() {
    let mut records: Vec<&str> = BOUND_RELATIONS
        .iter()
        .flat_map(|relation| relation.operations.iter())
        .filter_map(|&operation| lookup(operation))
        .flat_map(|standard| standard.request.into_iter().chain(Some(standard.result)))
        .collect();
    records.sort_unstable();
    records.dedup();
    assert_eq!(
        records.len(),
        8,
        "the request and result of four operations"
    );
    let dir = tempdir().expect("tempdir");
    for record in records {
        let mut bare = library_record(record);
        for (name, _) in TODAYS_BOUNDS {
            bare = bare.replace(&format!("[{name}="), "[");
        }
        assert_ne!(bare, library_record(record), "{record} names no bound");
        let path = dir.path().join(format!("{record}.scxml"));
        std::fs::write(&path, &bare).expect("write the bare record");
        let named = bound_constants(&document(record), &[]);
        assert_eq!(
            named,
            bound_constants(path.to_str().expect("a path of text"), &[]),
            "{record}: the named bounds declare what the bare numbers did"
        );
        let moved = bound_constants(&document(record), &["cose_payload=8192"]);
        let payload_moved = record.ends_with("sign1_request") || record.ends_with("verify1_result");
        assert_eq!(
            moved
                .iter()
                .any(|line| line.contains("PAYLOAD_MAX_SIZE: u32 = 8192;")),
            payload_moved,
            "{record}: --limit cose_payload=8192 moves the payload's bound: {moved:?}"
        );
        if !payload_moved {
            assert_eq!(moved, named, "{record}: a bound it does not name moved it");
        }
    }
}

// ── A record signed by name (docs/SCE_HOST_FEATURES.md, section 10; docs/adr/0109, D9) ──

/// The record a payload probe signs and verifies by name.
const NOTE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="note" sce:event-name="note.made">
  <datamodel>
    <data id="serial" sce:type="uint32" sce:direction="in"/>
    <data id="owner" sce:type="string[24]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// An enum a probe imports, which is no record.
const LEVEL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="0"/>
      <sce:variant name="high" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;

/// The description the build writes of [`NOTE`]'s type, as the runtimes'
/// record codecs read it, the fields in the order their keys encode, escaped
/// as a Rust and a C++ text literal both spell it.
const NOTE_DESCRIPTION_ESCAPED: &str =
    r#"{\"record\":[[\"owner\",false,{\"text\":24},null],[\"serial\",false,{\"uint\":32},null]]}"#;

const PAYLOAD_SIBLINGS: [(&str, &str); 2] = [("note.scxml", NOTE), ("level.scxml", LEVEL)];

/// A `sce-static` statechart that signs `payload` with a signing that carries
/// `sign_attrs`, and verifies the object with a verification that carries
/// `verify_attrs`, keeping what the verification read in `got`.
fn payload_probe(sign_attrs: &str, payload: &str, verify_attrs: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/cose_sign1_request.scxml" as="Sign1Request"/>
  <sce:import kind="event-schema" src="sce:std/host/cose_sign1_result.scxml" as="Sign1Result"/>
  <sce:import kind="event-schema" src="sce:std/host/cose_verify1_request.scxml" as="Verify1Request"/>
  <sce:import kind="event-schema" src="sce:std/host/cose_verify1_result.scxml" as="Verify1Result"/>
  <sce:import kind="event-schema" src="note.scxml" as="Note"/>
  <datamodel>
    <data id="note" sce:type="record:Note">
      <sce:set name="serial" expr="7"/>
      <sce:set name="owner" expr="'alice'"/>
    </data>
    <data id="label" sce:type="string[24]" expr="''"/>
    <data id="object" sce:type="bytes[cose_signed=4352]" expr="''"/>
    <data id="got" sce:type="optional&lt;record:Note&gt;" expr="null" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="signing"/>
  </state>
  <state id="signing">
    <invoke type="sce-host-cose-sign1" id="sign"{sign_attrs} sce:request="Sign1Request" sce:result="Sign1Result">
      <param name="key_id" expr="'k'"/>
      <param name="payload" expr="{payload}"/>
      <param name="external_aad" expr="''"/>
    </invoke>
    <transition event="done.invoke.sign" target="verifying">
      <assign location="object" expr="_event.data.cose"/>
    </transition>
  </state>
  <state id="verifying">
    <invoke type="sce-host-cose-verify1" id="verify"{verify_attrs} sce:request="Verify1Request" sce:result="Verify1Result">
      <param name="trust_set" expr="'t'"/>
      <param name="cose" expr="object"/>
      <param name="external_aad" expr="''"/>
    </invoke>
    <transition event="done.invoke.verify" target="idle">
      <assign location="got" expr="_event.data.payload"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// The probe that signs and verifies the note by name.
fn note_probe() -> String {
    payload_probe(r#" sce:payload="Note""#, "note", r#" sce:payload="Note""#)
}

/// docs/adr/0109, D0 and D9: a record is signed and verified by name on Rust and
/// C++, and a build for every other lane refuses the machine by name.
#[test]
fn a_record_signed_by_name_is_served_on_rust_and_cpp_and_refused_elsewhere() {
    let doc = note_probe();
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &PAYLOAD_SIBLINGS);
        if matches!(language, "rust" | "cpp") {
            assert!(ok, "{language} refused a record signed by name: {text}");
        } else {
            assert!(!ok, "{language} took a record signed by name: {text}");
            assert!(
                text.contains("codegen/standard-host-type-not-served")
                    && text.contains("sce-host-cose-sign1")
                    && text.contains("rust, cpp"),
                "{language}: {text}"
            );
        }
    }
}

/// The machine encodes the record where the signing starts and reads the record
/// back where the verification completes, by the description the build writes
/// of its type, and the failure of bytes that are no note is the build's own.
#[test]
fn the_build_encodes_the_record_where_a_signing_starts_and_reads_it_where_a_verification_ends() {
    let doc = note_probe();
    for (language, encode, open) in [
        (
            "rust",
            "::sce_rust_runtime::record_cbor::encode_text(",
            "::sce_rust_runtime::record_cbor::open_completion(",
        ),
        (
            "cpp",
            "::SCE::Common::RecordCbor::encodeValue(",
            "::SCE::Common::RecordCbor::openCompletion(",
        ),
    ] {
        let (ok, text, written) = generate_beside(language, &doc, &PAYLOAD_SIBLINGS);
        assert!(ok, "{language} did not generate: {text}");
        assert!(
            written.contains(encode),
            "{language}: the signing does not encode its record"
        );
        assert!(
            written.contains(open),
            "{language}: the verification does not read its record"
        );
        assert!(
            written.contains(NOTE_DESCRIPTION_ESCAPED),
            "{language}: the record's description is not the one its codec reads"
        );
        assert!(
            written.contains("not_a_record"),
            "{language}: the failure of bytes that are no note is not written"
        );
    }
}

/// `sce:payload` names an imported record, on a COSE operation that carries one;
/// a name of no import, an enum (which no record codec writes) and another
/// operation are each `validation/payload-record-schema`.
#[test]
fn a_record_named_by_sce_payload_is_imported_and_named_on_a_cose_operation() {
    for (doc, why) in [
        (
            payload_probe(r#" sce:payload="Memo""#, "note", r#" sce:payload="Note""#),
            "names no event schema this document imports",
        ),
        (
            payload_probe(r#" sce:payload="Level""#, "note", r#" sce:payload="Note""#).replace(
                r#"<sce:import kind="event-schema" src="note.scxml" as="Note"/>"#,
                r#"<sce:import kind="event-schema" src="note.scxml" as="Note"/>
  <sce:import kind="enum" src="level.scxml" as="Level"/>"#,
            ),
            "names no event schema this document imports",
        ),
        (
            kv_get("256", REQUEST, RESULT, "key").replace(
                r#"<invoke type="sce-host-kv-get" id="get""#,
                r#"<invoke type="sce-host-kv-get" id="get" sce:payload="KvGetResult""#,
            ),
            "sce-host-cose-sign1",
        ),
    ] {
        let (ok, text) = run(&["check"], &doc, &PAYLOAD_SIBLINGS);
        assert!(
            !ok,
            "a record the build cannot sign by name was accepted: {text}"
        );
        assert!(
            text.contains("validation/payload-record-schema") && text.contains(why),
            "{text}"
        );
    }
}

/// A record whose deterministic CBOR may be longer than the byte string it
/// crosses in holds is refused where it is named, naming the bound `--limit`
/// raises.
#[test]
fn a_record_whose_cbor_may_pass_its_bound_is_refused() {
    let (ok, text) = run(
        &["--limit", "cose_payload=16", "check"],
        &note_probe(),
        &PAYLOAD_SIBLINGS,
    );
    assert!(!ok, "a record past its bound was accepted: {text}");
    assert!(
        text.contains("validation/payload-record-schema")
            && text.contains("past the 16")
            && text.contains("cose_payload"),
        "{text}"
    );
    let (ok, text) = run(&["check"], &note_probe(), &PAYLOAD_SIBLINGS);
    assert!(ok, "the record fits the library's bound: {text}");
}

/// The payload of a signing that names its record is a value of that record,
/// and a text is refused where it is written.
#[test]
fn the_payload_of_a_signing_by_name_is_a_record_of_its_schema() {
    let doc = payload_probe(r#" sce:payload="Note""#, "label", r#" sce:payload="Note""#);
    let (ok, text) = run(&["check", "-l", "rust"], &doc, &PAYLOAD_SIBLINGS);
    assert!(!ok, "a text was taken for a record: {text}");
    assert!(
        text.contains("scxml/static-datamodel-rule") && text.contains("record:Note"),
        "{text}"
    );
}

/// The build describes the note of the fixture as the cross-language vector
/// does, so the two lanes' codecs, held to the vector, read what the build
/// writes; and a verification that names the note completes with the record
/// the build derives of the library's result.
#[test]
fn the_build_describes_the_fixtures_note_as_the_cross_language_vector_does() {
    use sce_build::forge::value_type::{TypeOf, ValueType};
    let fixture =
        repo_root().join("sce-build/tests/fixtures/static_datamodel/static_host_cose.scxml");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(fixture.to_str().expect("a utf-8 path"))
        .expect("the fixture parses");
    let described = sce_build::forge::payload_record::describe(
        &ValueType::Record {
            alias: "Note".to_string(),
        },
        &model.imported_type_scope,
    )
    .expect("the note has a CBOR form");
    let vectors: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("tests/host_features/payload_record_cross_language.json"),
        )
        .expect("the vector file is readable"),
    )
    .expect("the vector file is JSON");
    assert_eq!(described, vectors["description"].as_str().expect("a text"));
    let bound = model
        .imported_event_schemas
        .get("done.invoke.verify_note")
        .expect("the verification's completion is bound");
    let payload = bound
        .fields
        .iter()
        .find(|field| field.id == "payload")
        .expect("the result's payload");
    assert_eq!(
        payload.sce_type,
        TypeOf::Optional {
            elem: Box::new(TypeOf::Record {
                alias: "Note".to_string()
            })
        },
        "the machine receives the note, or none"
    );
}

/// The build describes the note's second revision
/// (`schema_cose_note_revised.scxml`) as the cross-language vector does: each
/// field the revision added with the bytes a note the first revision wrote
/// reads in its place, so the two lanes' codecs, held to the vector, read an
/// earlier revision's note as a machine built from the later schema does
/// (docs/adr/0044).
#[test]
fn the_build_describes_a_later_revision_of_the_note_as_the_cross_language_vector_does() {
    use sce_build::forge::value_type::ValueType;
    const REVISED: &str = "schema_cose_note_revised.scxml";
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        repo_root()
            .join("sce-build/tests/fixtures/static_datamodel")
            .join(REVISED),
        dir.path().join(REVISED),
    )
    .expect("copy the revised note");
    // A statechart that imports the record and runs nothing, read so the build
    // describes the record as a machine that imports it would.
    let holder = dir.path().join("holder.scxml");
    std::fs::write(
        &holder,
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="holder">
  <sce:import kind="event-schema" src="{REVISED}" as="Note"/>
  <state id="idle"/>
</scxml>
"#
        ),
    )
    .expect("write the holder");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(holder.to_str().expect("a utf-8 path"))
        .expect("the holder parses");
    let described = sce_build::forge::payload_record::describe(
        &ValueType::Record {
            alias: "Note".to_string(),
        },
        &model.imported_type_scope,
    )
    .expect("the revised note has a CBOR form");
    let vectors: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("tests/host_features/payload_record_cross_language.json"),
        )
        .expect("the vector file is readable"),
    )
    .expect("the vector file is JSON");
    assert_eq!(
        described,
        vectors["revised"]["description"]
            .as_str()
            .expect("the vector describes the revised note")
    );
}

/// The corpus that holds the runtimes' record codecs and the codecs ADR 0101
/// prints to one set of bytes (`codec_cbor_payload_record`) describes each of
/// its records as the build does, so the descriptions the runtimes are held
/// with are the ones a machine is generated with.
#[test]
fn the_build_describes_the_corpus_records_as_the_corpus_does() {
    use sce_build::forge::value_type::ValueType;
    let holder =
        repo_root().join("tests/forge/conformance/codec_cbor_composed/cbor_payload_holder.scxml");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(holder.to_str().expect("a utf-8 path"))
        .expect("the holder parses");
    let reference: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("tests/forge/conformance/numerical_reference.json"),
        )
        .expect("the corpus is readable"),
    )
    .expect("the corpus is JSON");
    let records = &reference["codecs"]["codec_cbor_payload_record"]["records"];
    for (alias, record) in [("Note", "note"), ("Leaves", "leaves")] {
        let described = sce_build::forge::payload_record::describe(
            &ValueType::Record {
                alias: alias.to_string(),
            },
            &model.imported_type_scope,
        )
        .unwrap_or_else(|why| panic!("{alias} is no record signed by name: {why}"));
        assert_eq!(
            described,
            records[record]["description"]
                .as_str()
                .expect("a description"),
            "{alias}"
        );
    }
}
