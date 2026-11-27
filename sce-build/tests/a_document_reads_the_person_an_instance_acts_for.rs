// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The person behind an instance, as a `sce-static` statechart reads it (docs/adr/0116, D3,
//! D4 and D10): `<scxml sce:principal="Person">` names an imported record schema whose
//! fields are taken by name from a fixed list, the document reads `_x.principal` and
//! `_x.quoted` as optional records of it, `sce:principal="required"` on a transition is the
//! guard `_x.principal !== null`, and `<send sce:on-behalf>` is refused until a deployment
//! lists the machines a machine takes a quote from. The Rust host core holds the person;
//! every other backend refuses the document by name.
//!
//! `backends/rust/tests/tests/a_static_server_reads_the_person_an_instance_acts_for.rs` runs
//! the fixture machine; this file holds what the build answers before anything runs.

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
    repo_root().join("sce-build/tests/fixtures/host_processor")
}

/// The fixture machine and the record it reads a person as, copied beside a probe.
const SIBLINGS: [&str; 2] = [
    "schema_principal_person.scxml",
    "statechart_static_reads_a_principal.scxml",
];

/// `sce-codegen <args…> <file>` in a directory holding the siblings and `documents`.
/// Returns `(exit_ok, stderr + stdout, the text of every file written)`.
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
fn every_backend() -> Vec<(sce_build::generator::Language, Vec<&'static str>)> {
    sce_build::forge::codegen_matrix::emitting(sce_build::forge::model::ForgeKind::Statechart)
        .map(|language| {
            let mut args = vec!["generate", "-l", language.canonical_name()];
            if language == sce_build::generator::Language::Go {
                args.extend(["--go-module-prefix", "example.test/generated"]);
            }
            (language, args)
        })
        .collect()
}

/// A `sce-static` statechart that imports the record `Person` as `schema` declares it,
/// declares `root` on its `<scxml>` and `body` as its one state's content.
fn statechart(schema: &str, root: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="principal_probe" {root}>
  <sce:import kind="event-schema" src="{schema}" as="Person"/>
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="idle">
    {body}
  </state>
</scxml>
"#
    )
}

/// An event schema `Person` of `fields`, each `(id, type)`.
fn person_schema(fields: &[(&str, &str)]) -> String {
    let data: String = fields
        .iter()
        .map(|(id, ty)| format!(r#"    <data id="{id}" sce:type="{ty}" sce:direction="in"/>\n"#))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="probe_person" sce:event-name="probe.person">
  <datamodel>
{data}  </datamodel>
</scxml>
"#
    )
}

/// `documents` checked for Rust, with `probe.scxml` the statechart: refused with `code`,
/// the refusal holding `needle`.
fn refused(documents: &[(&str, &str)], code: &str, needle: &str) {
    let (ok, out, _) = run(&["check", "-l", "rust"], documents, "probe.scxml");
    assert!(!ok, "expected a refusal with {code}:\n{out}");
    assert!(out.contains(code), "expected the code {code} in:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

/// The fixture machine is generated for Rust: it holds the person the event being processed
/// was said for in an optional value of the record the document declares, taken from the
/// event's metadata field by field, each text held to its bound (`method` and `instance`
/// are not declared, and are not given), and the quoted person, which no event carries.
/// Red if the lowering reads another place, or gives a field the record does not declare.
#[test]
fn the_rust_machine_holds_the_person_its_event_was_said_for() {
    let (ok, out, written) = run(
        &["generate", "-l", "rust"],
        &[],
        "statechart_static_reads_a_principal.scxml",
    );
    assert!(ok, "the Rust machine is generated:\n{out}");
    for needle in [
        "pending_event_principal: Option<",
        "pending_event_quoted: Option<",
        "fn admit_principal(",
        "issuer: ::sce_rust_runtime::portable::SceString::from_text(&person.issuer)",
        "subject: ::sce_rust_runtime::portable::SceString::from_text(&person.subject)",
        "proved_ms: person.proved_ms",
        "self.pending_event_quoted = None;",
    ] {
        assert!(
            written.contains(needle),
            "expected `{needle}` in:\n{written}"
        );
    }
    for absent in ["person.method", "person.instance"] {
        assert!(!written.contains(absent), "`{absent}` is given:\n{written}");
    }
}

/// docs/adr/0116, D10: every backend but Rust refuses the document by name, where its
/// generation begins, and so does the Interpreter's lowering. Red if a backend generates a
/// machine that reads a person no core of its holds.
#[test]
fn every_other_backend_refuses_a_document_that_reads_a_person() {
    for (language, args) in every_backend() {
        if language == sce_build::generator::Language::Rust {
            continue;
        }
        let (ok, out, _) = run(&args, &[], "statechart_static_reads_a_principal.scxml");
        assert!(!ok, "{language:?} generates it:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature") && out.contains("sce:principal"),
            "{language:?}:\n{out}"
        );
    }
    let (ok, out, _) = run(&["lower"], &[], "statechart_static_reads_a_principal.scxml");
    assert!(!ok, "the Interpreter's lowering lowers it:\n{out}");
    assert!(out.contains("sce:principal"), "{out}");
}

/// docs/adr/0116, D3: the record a person is read as is an imported event schema whose every
/// field is one of the fixed list, of its type; and a document reads a person under
/// `sce-static` alone. Each is `validation/principal-record`. Red if
/// `check_declaration` or `declared` takes any of them.
#[test]
fn the_record_a_person_is_read_as_takes_its_fields_from_the_fixed_list() {
    let code = "validation/principal-record";
    let probe = || {
        statechart(
            "person.scxml",
            r#"sce:principal="Person""#,
            r#"<transition event="go"/>"#,
        )
    };
    for (fields, needle) in [
        (
            vec![("subject", "string[64]"), ("email", "string[64]")],
            "`email`",
        ),
        (vec![("subject", "uint32")], "`subject` as `uint32`"),
        (vec![("proved_ms", "int64")], "`proved_ms` as `int64`"),
    ] {
        refused(
            &[
                ("person.scxml", &person_schema(&fields)),
                ("probe.scxml", &probe()),
            ],
            code,
            needle,
        );
    }
    refused(
        &[
            ("person.scxml", &person_schema(&[("subject", "string[64]")])),
            (
                "probe.scxml",
                &statechart(
                    "person.scxml",
                    r#"sce:principal="Someone""#,
                    r#"<transition event="go"/>"#,
                ),
            ),
        ],
        code,
        "no imported event schema",
    );
    refused(
        &[
            ("person.scxml", &person_schema(&[("subject", "string[64]")])),
            (
                "probe.scxml",
                &statechart(
                    "person.scxml",
                    r#"sce:principal="Person""#,
                    r#"<transition event="go"/>"#,
                )
                .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#),
            ),
        ],
        code,
        "sce-static",
    );
    refused(
        &[
            ("person.scxml", &person_schema(&[("subject", "string[64]")])),
            (
                "probe.scxml",
                &statechart(
                    "person.scxml",
                    "",
                    r#"<transition event="go" sce:principal="required"/>"#,
                ),
            ),
        ],
        code,
        "declares no person",
    );
}

/// `sce:principal` on a transition takes `required` alone; and a read of `_x.principal` is
/// held to the rules an optional record is, its field read only where the record is proven
/// present. Red if the attribute takes another value, or if the person is read unproven.
#[test]
fn a_transition_requires_a_person_and_reads_one_only_where_it_is_proven() {
    let schema = person_schema(&[("subject", "string[64]")]);
    refused(
        &[
            ("person.scxml", &schema),
            (
                "probe.scxml",
                &statechart(
                    "person.scxml",
                    r#"sce:principal="Person""#,
                    r#"<transition event="go" sce:principal="maybe"/>"#,
                ),
            ),
        ],
        "validation/attribute-rule-violated",
        "required",
    );
    refused(
        &[
            ("person.scxml", &schema),
            (
                "probe.scxml",
                &statechart(
                    "person.scxml",
                    r#"sce:principal="Person""#,
                    r#"<transition event="go" cond="_x.principal.value.subject === 'alice'"/>"#,
                ),
            ),
        ],
        "expression/optional-read-unproven",
        "_x.principal",
    );
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[
            ("person.scxml", &schema),
            (
                "probe.scxml",
                &statechart(
                    "person.scxml",
                    r#"sce:principal="Person""#,
                    r#"<transition event="go" sce:principal="required" cond="_x.principal.value.subject === 'alice'"/>"#,
                ),
            ),
        ],
        "probe.scxml",
    );
    assert!(ok, "the required guard proves the person present:\n{out}");
}

/// docs/adr/0116, D4 and Open question 2: a quote is refused until a deployment lists the
/// machines a machine takes one from, and the refusal names what the attribute is held to
/// first. Each is `validation/quote-refused`. Red if `refuse_quote` lets a send quote.
#[test]
fn every_quote_is_refused_until_a_deployment_lists_one() {
    let schema = person_schema(&[("subject", "string[64]")]);
    for (root, behalf, needle) in [
        (
            r#"sce:principal="Person""#,
            "_x.principal",
            "no deployment lists",
        ),
        (r#"sce:principal="Person""#, "count", "the one authority"),
        ("", "_x.principal", "declares no person"),
    ] {
        refused(
            &[
                ("person.scxml", &schema),
                (
                    "probe.scxml",
                    &statechart(
                        "person.scxml",
                        root,
                        &format!(
                            r##"<transition event="go"><send event="ask" target="#_internal" sce:on-behalf="{behalf}"/></transition>"##
                        ),
                    ),
                ),
            ],
            "validation/quote-refused",
            needle,
        );
    }
}
