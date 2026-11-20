// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value the deployment supplies (docs/adr/0109, D12; docs/SCE_ACCEPTED_SUBSET.md
// §2.15, "A value the deployment supplies"): `<data sce:from="deployment">` declares a
// value of a type the deployment gives the machine before it starts. The machine
// reads it and never writes it, so every statement that would write it is refused on
// its own code and on the row of the element; its declaration takes no initial value
// of the document's; and Rust and C++ build the machine from it while every other
// backend, and the Interpreter's lowering, refuses it by name. A configuration the
// deployment writes under the machine (docs/SCE_ACCEPTED_SUBSET.md §2.8.4) is checked
// against the document when the machine is generated, each refusal on its own code, and
// written beside the machine as the JSON its reader reads.

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Run `sce-codegen <args…> probe.scxml` with JSON diagnostics, `siblings` written
/// beside the document where its imports resolve, and return `(exit_ok, output)`.
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

/// The record the deployment's `limits` is a value of.
const LIMITS_SCHEMA: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="limits" sce:event-name="limits">
  <datamodel>
    <data id="max_rows" sce:type="uint32" sce:direction="in"/>
    <data id="label" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A child whose own value the deployment would supply.
const DEPLOYED_CHILD: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="run" datamodel="sce-static" name="child">
  <datamodel>
    <data id="limit" sce:type="uint32" sce:from="deployment"/>
  </datamodel>
  <state id="run"/>
</scxml>
"#;

fn siblings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("limits.scxml", LIMITS_SCHEMA),
        ("child.scxml", DEPLOYED_CHILD),
    ]
}

/// A statechart under `datamodel` whose deployment supplies a record and a list, whose
/// `limits` declaration ends with `extra`, and whose `go` transition holds `body`. A
/// statement of the body stands on line [`BODY_LINE`]; the `limits` declaration on
/// line [`LIMITS_LINE`].
fn statechart(datamodel: &str, extra: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}" name="probe">
  <sce:import kind="event-schema" src="limits.scxml" as="Limits"/>
  <datamodel>
    <data id="limits" sce:type="record:Limits" sce:from="deployment"{extra}/>
    <data id="hosts" sce:type="list&lt;string[32]&gt;[4]" sce:from="deployment"/>
    <data id="rows" sce:type="uint32" expr="limits.max_rows"/>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
{body}
    </transition>
  </state>
</scxml>
"#
    )
}

const LIMITS_LINE: u32 = 6;
const BODY_LINE: u32 = 13;

/// A body that reads both values and writes neither.
const READS: &str =
    r#"      <assign location="count" expr="count + limits.max_rows + len(hosts)"/>"#;

fn check(args: &[&str], doc: &str) -> (bool, String) {
    run(args, doc, &siblings())
}

/// Assert a refusal carries `code` and is placed on `line`.
fn assert_refused_at(out: &str, code: &str, line: u32) {
    let record = out
        .lines()
        .find(|l| l.contains(&format!("\"code\":\"{code}\"")))
        .unwrap_or_else(|| panic!("expected {code}, got:\n{out}"));
    assert!(
        record.contains(&format!("\"line\":{line},"))
            || record.contains(&format!("\"line\":{line}}}")),
        "{code} must be placed on line {line}:\n{record}"
    );
}

/// A machine that reads what its deployment supplies, an initial value among the
/// reads, keeps every rule and is lowered by Rust and C++.
#[test]
fn a_value_the_deployment_supplies_is_read_and_lowered_by_rust_and_cpp() {
    let doc = statechart("sce-static", "", READS);
    let (ok, out) = check(&["check"], &doc);
    assert!(
        ok,
        "the judge takes a value the deployment supplies:\n{out}"
    );
    for language in ["rust", "cpp"] {
        let (ok, out) = check(&["check", "-l", language], &doc);
        assert!(
            ok,
            "{language} builds the machine from its deployment:\n{out}"
        );
    }
}

/// Every statement that writes a place, held to a place inside a deployment value:
/// each is refused as `validation/deployment-value-assigned`, on its own row, with the
/// place as written as the record's `actual`.
#[test]
fn every_statement_that_writes_a_deployment_value_is_refused() {
    let writes = [
        (
            r#"      <assign location="limits.max_rows" expr="1"/>"#,
            "limits.max_rows",
        ),
        (
            r#"      <assign location="limits" expr="limits"/>"#,
            "limits",
        ),
        (r#"      <sce:append target="hosts" expr="'h'"/>"#, "hosts"),
        (r#"      <sce:clear target="hosts"/>"#, "hosts"),
        (
            r#"      <sce:put target="hosts" key="1" expr="'h'"/>"#,
            "hosts",
        ),
        (r#"      <sce:remove target="hosts" key="1"/>"#, "hosts"),
        (
            r#"      <foreach array="hosts" item="limits"><assign location="count" expr="1"/></foreach>"#,
            "limits",
        ),
        (
            r##"      <send event="go" target="#_internal" idlocation="limits"/>"##,
            "limits",
        ),
    ];
    for (body, place) in writes {
        let (ok, out) = check(&["check"], &statechart("sce-static", "", body));
        assert!(!ok, "a write of `{place}` is refused:\n{out}");
        assert_refused_at(&out, "validation/deployment-value-assigned", BODY_LINE);
        assert!(
            out.contains(&format!("\"actual\":\"{place}\"")),
            "the place written is the record's `actual`:\n{out}"
        );
    }
}

/// The declaration names the deployment as its one source, and takes nothing the
/// document would start it at; it is the host's input, so it is not published.
#[test]
fn a_declaration_of_a_deployment_value_holds_its_rules() {
    // An initial value of the document's, and a publication to the host.
    for extra in [r#" expr="limits""#, r#" sce:direction="out""#] {
        let (ok, out) = check(&["check"], &statechart("sce-static", extra, READS));
        assert!(!ok, "`{extra}` beside sce:from is refused:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", LIMITS_LINE);
    }
    // A source other than the deployment.
    let doc = statechart("sce-static", "", READS).replace(
        r#"sce:type="record:Limits" sce:from="deployment""#,
        r#"sce:type="record:Limits" sce:from="environment""#,
    );
    let (ok, out) = check(&["check"], &doc);
    assert!(!ok, "a source other than the deployment is refused:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", LIMITS_LINE);
    assert!(out.contains("environment"), "{out}");
    // A field built by `<sce:set>`, as a record variable of the document is.
    let doc = statechart("sce-static", "", READS).replace(
        r#"sce:type="record:Limits" sce:from="deployment"/>"#,
        r#"sce:type="record:Limits" sce:from="deployment"><sce:set name="max_rows" expr="1"/><sce:set name="label" expr="''"/></data>"#,
    );
    let (ok, out) = check(&["check"], &doc);
    assert!(
        !ok,
        "a deployment value built by <sce:set> is refused:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", LIMITS_LINE);
    // A map, which has no wire form a configuration could be read from
    // (docs/adr/0108, D7).
    let doc = statechart("sce-static", "", READS).replace(
        r#"sce:type="record:Limits" sce:from="deployment""#,
        r#"sce:type="map&lt;uint32, bool&gt;[4]" sce:from="deployment""#,
    );
    let (ok, out) = check(&["check"], &doc);
    assert!(!ok, "a map the deployment would supply is refused:\n{out}");
    assert!(
        out.contains("\"code\":\"scxml/static-datamodel-rule\"") && out.contains("payload schema"),
        "a map has no payload schema:\n{out}"
    );
}

/// Under any other data model the attribute is read by nothing, and refused so.
#[test]
fn a_deployment_value_under_another_data_model_is_unread() {
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel>
    <data id="limit" expr="1" sce:from="deployment"/>
  </datamodel>
  <state id="s"/>
</scxml>
"#;
    let (ok, out) = check(&["check"], doc);
    assert!(!ok, "sce:from outside sce-static is refused:\n{out}");
    assert!(
        out.contains("\"code\":\"validation/sce-attribute-unread\""),
        "the attribute is unread there:\n{out}"
    );
}

/// A child session is started by its parent, which has no deployment value to give
/// it: an `<invoke>` of a child that declares one is refused at the invoke.
#[test]
fn a_child_that_declares_a_deployment_value_is_refused_at_its_invoke() {
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <invoke id="worker" type="scxml" src="child.scxml"/>
  </state>
</scxml>
"#;
    let (ok, out) = check(&["check"], doc);
    assert!(!ok, "a child with a deployment value is refused:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(out.contains("limit"), "the child's value is named:\n{out}");
}

/// Rust and C++ read a value the deployment supplies (docs/adr/0109, D0); every other
/// backend, and the Interpreter's lowering, refuses it by name where the `<data>` is
/// walked, rather than starting the machine on a value no deployment gave.
#[test]
fn every_other_backend_refuses_a_deployment_value_by_name() {
    // A number alone, which every backend lowers as a variable, so the refusal is
    // the deployment value's and no other construct's.
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="limit" sce:type="uint32" sce:from="deployment"/>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="count" expr="count + limit"/>
    </transition>
  </state>
</scxml>
"#;
    for language in Language::ALL {
        let name = language.canonical_name();
        let (ok, out) = check(&["check", "-l", name], doc);
        match language {
            Language::Rust | Language::Cpp => {
                assert!(ok, "{name} reads a deployment value:\n{out}");
            }
            _ => {
                assert!(!ok, "{name} reads no deployment value yet:\n{out}");
                assert!(
                    out.contains("\"code\":\"generate/unsupported-feature\""),
                    "{name} refuses it by name:\n{out}"
                );
                assert!(
                    out.contains("a value a deployment supplies has no"),
                    "the refusal says what {name} does not read:\n{out}"
                );
            }
        }
    }
    let (ok, out) = check(&["lower"], doc);
    assert!(!ok, "the Interpreter reads no deployment value:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature")
            && out.contains("a value a deployment supplies has no ecmascript lowering"),
        "the Interpreter refuses it by name:\n{out}"
    );
}

/// A deployment of the machine `probe` whose entry ends with `lines`.
fn deployment(lines: &str) -> String {
    format!(
        "version: \"1.0\"\ntopology:\n  host:\n    machines:\n      probe:\n        source: \
         probe.scxml\n{lines}"
    )
}

/// The configuration of [`statechart`]'s machine, written in the deployment.
const CONFIG: &str =
    "        config:\n          limits: { label: main, max_rows: 200 }\n          \
                      hosts: [a.example, b.example]\n";

/// `generate -l <language> --deploy` of [`statechart`]'s machine, which reads its two
/// values, with `deploy` as its deployment: whether it succeeded, what it said, and
/// the configuration the build wrote beside the machine, if it wrote one.
fn generate_deployed(language: &str, deploy: &str) -> (bool, String, Option<String>) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings() {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, statechart("sce-static", "", READS)).expect("write probe");
    let deploy_path = dir.path().join("deploy.yaml");
    std::fs::write(&deploy_path, deploy).expect("write deploy");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("out dir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("generate")
        .arg("-l")
        .arg(language)
        .arg("-o")
        .arg(&out_dir)
        // The configuration is what is asked of the run, not the C++ formatting.
        .arg("--no-format")
        .arg("--deploy")
        .arg(&deploy_path)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    let config = std::fs::read_to_string(out_dir.join("probe.config.json")).ok();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        config,
    )
}

/// A configuration written under the machine is checked against the values its
/// document declares and written beside the machine as the JSON its reader reads: one
/// object of the values by id, in declaration order, each record's fields in the
/// order its schema declares them, whatever order the deployment wrote them in. The
/// manifest lists each value with its type.
#[test]
fn a_configuration_in_the_deployment_is_written_beside_the_machine() {
    for language in ["rust", "cpp"] {
        let (ok, out, config) = generate_deployed(language, &deployment(CONFIG));
        assert!(ok, "{language} takes the configuration:\n{out}");
        assert_eq!(
            config.as_deref().map(str::trim_end),
            Some(r#"{"limits":{"max_rows":200,"label":"main"},"hosts":["a.example","b.example"]}"#),
            "{language}:\n{out}"
        );
        assert!(
            out.contains(r#""deployment":[{"id":"limits","type":"record:Limits"}"#),
            "the manifest lists each value with its type:\n{out}"
        );
    }
}

/// A file the deployment names is the host's to read at start: the build checks only
/// that the machine reads a configuration, and writes none of its own.
#[test]
fn a_configuration_file_is_left_to_the_host() {
    let (ok, out, config) = generate_deployed(
        "rust",
        &deployment("        config_file: /etc/probe/config.json\n"),
    );
    assert!(ok, "a file the host reads is accepted:\n{out}");
    assert!(
        config.is_none(),
        "the build writes no configuration of its own"
    );
}

/// Each refusal of a configuration the deployment writes, on its own code: a value
/// the document declares and the deployment leaves out, a key the document declares no
/// value of, and a value that is no value of its type by its payload schema.
#[test]
fn each_refusal_of_a_configuration_has_its_own_code() {
    for (lines, code, actual) in [
        (
            "        config:\n          limits: { label: main, max_rows: 200 }\n",
            "mesh/deploy-config-missing",
            "hosts",
        ),
        ("", "mesh/deploy-config-missing", "limits"),
        (
            "        config:\n          limits: { label: main, max_rows: 200 }\n          \
             hosts: []\n          host: [a]\n",
            "mesh/deploy-config-unknown",
            "host",
        ),
        (
            "        config:\n          limits: { label: main, max_rows: -1 }\n          \
             hosts: []\n",
            "mesh/deploy-config-invalid",
            "config/limits/max_rows",
        ),
        (
            "        config:\n          limits: { label: main, max_rows: 1, extra: 2 }\n          \
             hosts: []\n",
            "mesh/deploy-config-invalid",
            "config/limits/extra",
        ),
        (
            "        config:\n          limits: { label: main, max_rows: 1 }\n          \
             hosts: [a, b, c, d, e]\n",
            "mesh/deploy-config-invalid",
            "config/hosts",
        ),
    ] {
        let (ok, out, config) = generate_deployed("rust", &deployment(lines));
        assert!(!ok, "`{lines}` is refused:\n{out}");
        assert!(
            out.contains(&format!("\"code\":\"{code}\"")),
            "`{lines}` is {code}:\n{out}"
        );
        assert!(
            out.contains(&format!("\"actual\":\"{actual}\"")),
            "`{lines}` names `{actual}`:\n{out}"
        );
        assert!(config.is_none(), "a refused configuration is not written");
    }
}

/// The directory of the statechart fixtures, where `static_deployment.scxml` and its
/// `deployment/` stand.
fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The configuration `deployment/deploy.yaml` writes under `static_deployment` is the
/// text the Rust and C++ machines are built from in their own tests
/// (`a_machine_reads_its_deployments_values.rs`,
/// `AMachineReadsItsDeploymentsValuesTest.cpp`): what the build writes beside each
/// machine is the committed `deployment/static_deployment.config.json`, byte for byte.
#[test]
fn the_build_writes_the_configuration_the_machines_read() {
    let kept = std::fs::read_to_string(fixtures().join("deployment/static_deployment.config.json"))
        .expect("the committed configuration");
    for language in ["rust", "cpp"] {
        let dir = tempdir().expect("tempdir");
        let out = Command::new(sce_codegen_bin())
            .arg("--workspace-root")
            .arg(repo_root())
            .arg("--error-format=json")
            .arg("generate")
            .arg("-l")
            .arg(language)
            .arg("-o")
            .arg(dir.path())
            .arg("--no-format")
            .arg("--deploy")
            .arg(fixtures().join("deployment/deploy.yaml"))
            .arg(fixtures().join("static_deployment.scxml"))
            .output()
            .expect("invoke sce-codegen");
        let said = String::from_utf8_lossy(&out.stderr).into_owned()
            + &String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{language}:\n{said}");
        let written = std::fs::read_to_string(dir.path().join("static_deployment.config.json"))
            .unwrap_or_else(|e| panic!("{language} wrote no configuration ({e}):\n{said}"));
        assert_eq!(
            written, kept,
            "{language}: the build writes the committed text"
        );
    }
}
