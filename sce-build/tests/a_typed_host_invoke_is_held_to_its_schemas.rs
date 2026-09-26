// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A host-run `<invoke>`'s typed interface — `sce:request` / `sce:result`
//! (SCE Accepted Subset §2.12) — is refused at parse wherever it could not
//! mean what it says, on the row that shows why.
//!
//! Each case writes the statechart beside the two schemas it imports and
//! parses it from disk, because the aliases resolve through sibling files —
//! the path the CLI takes. The accepted case comes first: without it, every
//! refusal below could be the document failing for a reason of its own.

use std::path::PathBuf;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::parser::SCXMLParser;

const REQUEST_SCHEMA: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="perm_request"
       sce:kind="event-schema"
       sce:event-name="perm.request">
  <datamodel>
    <data id="scope" sce:type="string" sce:direction="in"/>
    <data id="retries" sce:type="int32" sce:direction="in"/>
  </datamodel>
</scxml>"#;

const RESULT_SCHEMA: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="perm_result"
       sce:kind="event-schema"
       sce:event-name="perm.result">
  <datamodel>
    <data id="granted" sce:type="bool" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// A statechart whose one state runs `invoke`, importing both schemas.
fn statechart(invoke: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="asking" version="1.0" initial="asking" datamodel="ecmascript">
  <sce:import kind="event-schema" src="perm_request.scxml" as="PermRequest"/>
  <sce:import kind="event-schema" src="perm_result.scxml" as="PermResult"/>
  <state id="asking">
{invoke}
  </state>
</scxml>"#
    )
}

/// Parse `invoke` in its statechart from a fresh directory of its own.
fn parse(case: &str, invoke: &str) -> Result<(), (String, String, Option<u32>)> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("a_typed_host_invoke_is_held_to_its_schemas")
        .join(case);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    std::fs::write(dir.join("perm_request.scxml"), REQUEST_SCHEMA).expect("write schema");
    std::fs::write(dir.join("perm_result.scxml"), RESULT_SCHEMA).expect("write schema");
    let path = dir.join("asking.scxml");
    std::fs::write(&path, statechart(invoke)).expect("write statechart");
    SCXMLParser::new()
        .parse_file(path.to_str().expect("utf-8 path"))
        .map(|_| ())
        .map_err(|refusal| {
            let diagnostic = &refusal.error.to_diagnostics()[0];
            (
                serde_json::to_value(diagnostic.code)
                    .expect("code serializes")
                    .as_str()
                    .expect("code is a string")
                    .to_string(),
                diagnostic.actual.clone().unwrap_or_default(),
                refusal.location.line,
            )
        })
}

/// The row of `needle` within the statechart `invoke` is placed in.
fn row_of(invoke: &str, needle: &str) -> Option<u32> {
    let text = statechart(invoke);
    let at = text.find(needle).expect("needle is in the document");
    Some(text[..at].matches('\n').count() as u32 + 1)
}

const TYPED: &str = r#"    <invoke type="x-app-host" id="perm" sce:request="PermRequest" sce:result="PermResult">
      <param name="scope" expr="'calendar'"/>
      <param name="retries" expr="2"/>
      <param name="_sce_deadline_ms" expr="500"/>
    </invoke>"#;

#[test]
fn a_request_whose_params_are_the_schemas_fields_is_accepted() {
    // The discriminator: the reserved deadline param is the engine's, not a
    // field, and is not counted against the record.
    parse("accepted", TYPED).expect("a typed invoke whose params match its schema parses");
}

#[test]
fn a_typed_interface_on_an_invoke_sce_runs_itself_is_refused() {
    let invoke = r#"    <invoke type="scxml" id="child" src="child.scxml"
            sce:result="PermResult"/>"#;
    let (code, actual, line) = parse("scxml", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-schema");
    assert_eq!(actual, "PermResult");
    assert_eq!(
        line,
        row_of(invoke, "sce:result"),
        "reported on the attribute's row"
    );
}

#[test]
fn a_typed_invoke_without_an_id_is_refused() {
    let invoke = r#"    <invoke type="x-app-host" sce:result="PermResult"/>"#;
    let (code, actual, _) = parse("no_id", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-schema");
    assert_eq!(actual, "PermResult");
}

#[test]
fn an_alias_no_import_declares_is_refused_on_its_own_row() {
    let invoke = r#"    <invoke type="x-app-host" id="perm"
            sce:result="PermResultt"/>"#;
    let (code, actual, line) = parse("unknown_alias", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-schema");
    assert_eq!(actual, "PermResultt");
    assert_eq!(
        line,
        row_of(invoke, "sce:result"),
        "reported on the attribute's row"
    );
}

#[test]
fn a_param_the_schema_lacks_is_refused_on_the_params_row() {
    let invoke = r#"    <invoke type="x-app-host" id="perm" sce:request="PermRequest">
      <param name="scope" expr="'calendar'"/>
      <param name="retries" expr="2"/>
      <param name="colour" expr="'red'"/>
    </invoke>"#;
    let (code, actual, line) = parse("extra_param", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-request");
    assert_eq!(actual, "colour");
    assert_eq!(
        line,
        row_of(invoke, "\"colour\""),
        "reported on the param's row"
    );
}

#[test]
fn a_field_no_param_supplies_is_refused() {
    let invoke = r#"    <invoke type="x-app-host" id="perm" sce:request="PermRequest">
      <param name="scope" expr="'calendar'"/>
    </invoke>"#;
    let (code, actual, line) = parse("missing_field", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-request");
    assert_eq!(actual, "PermRequest");
    assert_eq!(
        line,
        row_of(invoke, "sce:request"),
        "reported on the alias's row"
    );
}

#[test]
fn a_field_given_twice_is_refused() {
    let invoke = r#"    <invoke type="x-app-host" id="perm" sce:request="PermRequest">
      <param name="scope" expr="'calendar'"/>
      <param name="retries" expr="2"/>
      <param name="retries" expr="3"/>
    </invoke>"#;
    let (code, actual, _) = parse("twice", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-request");
    assert_eq!(actual, "retries");
}

#[test]
fn a_namelist_beside_a_typed_request_is_refused() {
    let invoke = r#"    <invoke type="x-app-host" id="perm" sce:request="PermRequest" namelist="scope">
      <param name="scope" expr="'calendar'"/>
      <param name="retries" expr="2"/>
    </invoke>"#;
    let (code, _, _) = parse("namelist", invoke).expect_err("refused");
    assert_eq!(code, "validation/typed-invoke-request");
}
