// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A machine holds the person an instance acts for wherever one can be proved to it
//! (docs/adr/0116, decision 3 of "Decided while making a person cost nothing where none is
//! read", as replaced on 2026-11-28). A machine whose document declares no `sce:principal`
//! holds the person its events and runs are said for when its deployment binds it where a
//! person is proved (D10: to an open machine's instances, under the `key` authenticator,
//! with `sign:` and `verify:`, receiving durably), so that who did what is recorded by the
//! platform (D7); it holds a slot of no size everywhere else, where no person can be told to
//! it; and what its document reads stays D3's.
//!
//! `a_document_reads_the_person_an_instance_acts_for.rs` holds the machine whose document
//! reads a person, and a machine generated with no deployment, which holds none.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::model::SCXMLModel;
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

/// The server `rooms`, whose document declares no `sce:principal`: it counts the `ping`s
/// its instances send, under the guard `cond` (none when empty).
fn rooms(cond: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="rooms">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="ping"{cond}>
      <assign location="count" expr="count + 1"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// The open machine `phone`, whose instances send `ping` to `rooms`.
const PHONE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" initial="idle" datamodel="sce-static" name="phone">
  <state id="idle">
    <onentry><send target="#rooms" event="ping"/></onentry>
  </state>
</scxml>
"##;

/// `rooms` bound to the instances of the open machine `phone` under the `key` authenticator,
/// with `sign:` and `verify:`, both checkpointed; `receipt` follows the keys of the
/// binding of `rooms`.
fn deployment(receipt: &str) -> String {
    format!(
        r##"version: "1.0"
topology:
  cloud:
    machines:
      rooms:
        source: rooms.scxml
        persistence: checkpoint
        bindings:
          "#phone": {{ transport: https, listen: "127.0.0.1:9443", authenticator: key, fresh_ms: 60000, sign: rooms-key{receipt} }}
  handset:
    machines:
      phone:
        source: phone.scxml
        instances: open
        members: {{ lease_ms: 60000, provisional_ms: 1000, max_instances: 4 }}
        persistence: checkpoint
        bindings:
          "#rooms": {{ transport: https, url: "https://rooms.example", authenticator: key, verify: rooms-trust }}
"##
    )
}

/// The deployment that proves a person to `rooms` (D10).
fn proving() -> String {
    deployment(", receipt: durable")
}

/// The same deployment, but `rooms` does not receive durably, so no person is proved to it:
/// an event could be told the person of a later incarnation than the one it was taken in
/// under (decision 4 of step 4).
fn not_proving() -> String {
    deployment("")
}

/// `sce-codegen generate -l rust` of `rooms` as `document` holds it, with `phone` beside it,
/// under `deploy` when one is given. Returns `(exit_ok, stderr + stdout, the text of every
/// file written)`.
fn generate(document: &str, deploy: Option<&str>) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("rooms.scxml"), document).expect("write rooms");
    std::fs::write(dir.path().join("phone.scxml"), PHONE).expect("write phone");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "rust"])
        .arg(dir.path().join("rooms.scxml"))
        .arg("-o")
        .arg(out_dir.path());
    if let Some(deploy) = deploy {
        let path = dir.path().join("deploy.yaml");
        std::fs::write(&path, deploy).expect("write the deployment");
        command.arg("--deploy").arg(path);
    }
    let out = command.output().expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written(out_dir.path()),
    )
}

/// The text of every file under `dir`, its subdirectories included.
fn written(dir: &Path) -> String {
    let mut text = String::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(at) = pending.pop() {
        for entry in std::fs::read_dir(&at).expect("a directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
                text.push('\n');
            }
        }
    }
    text
}

/// The diagnostic codes `out` names, in order.
fn codes(out: &str) -> Vec<String> {
    out.split("\"code\":\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect()
}

/// The slot of a machine that holds the person its events and runs are said for, and its
/// external queue, which carries the person beside each event.
const HOLDS: [&str; 2] = [
    "type Principal = Option<Box<::sce_rust_runtime::Principal>>;",
    "{ sce_rust_runtime::MAX_EVENT_QUEUE_DEPTH }, Self::Principal>;",
];

/// What a machine whose document reads a person has, and one whose document reads none does
/// not: the door the person comes in by, and the place `_x.principal` reads.
const READS: [&str; 2] = ["fn admit_principal(", "pending_event_principal"];

/// The slot of no size.
const HOLDS_NONE: &str = "type Principal = ::sce_rust_runtime::NoPrincipal;";

/// docs/adr/0116, D7 and D10: `rooms`, whose document declares no `sce:principal`, bound
/// where a person is proved, holds the person its events and runs are said for, and takes
/// none into anything its document reads; bound where none is proved, it holds a slot of no
/// size. Red if `apply_deploy_model_mutations` does not mark the proof, or the template
/// chooses the slot from what the document reads alone.
#[test]
fn a_machine_bound_where_a_person_is_proved_holds_one_its_document_does_not_read() {
    let (ok, out, written) = generate(&rooms(""), Some(&proving()));
    assert!(ok, "rooms is generated under the proving binding:\n{out}");
    for needle in HOLDS {
        assert!(
            written.contains(needle),
            "expected `{needle}` in:\n{written}"
        );
    }
    for absent in READS {
        assert!(
            !written.contains(absent),
            "`{absent}` is generated for a document that reads no person:\n{written}"
        );
    }

    let (ok, out, written) = generate(&rooms(""), Some(&not_proving()));
    assert!(ok, "rooms is generated where no person is proved:\n{out}");
    assert!(
        written.contains(HOLDS_NONE),
        "expected `{HOLDS_NONE}` in:\n{written}"
    );
    assert!(!written.contains(HOLDS[0]), "a person's slot:\n{written}");
}

/// docs/adr/0116, D3: the slot a deployment gives the machine lets its document read nothing
/// it did not declare. `rooms` reading `_x.principal` with no `sce:principal` is refused
/// under the proving binding as it is with no deployment, by each code it is refused by there.
/// Red if the
/// proof of a person reaches what the document's expressions are typed by.
#[test]
fn what_a_document_reads_is_not_widened_by_the_person_its_machine_holds() {
    let reading = rooms(r#" cond="_x.principal !== null""#);
    let (ok, alone, _) = generate(&reading, None);
    assert!(!ok, "an undeclared read of the person is refused:\n{alone}");
    let (ok, deployed, _) = generate(&reading, Some(&proving()));
    assert!(
        !ok,
        "an undeclared read of the person is refused where one is proved:\n{deployed}"
    );
    let refused = codes(&alone);
    assert!(!refused.is_empty(), "a coded refusal:\n{alone}");
    let under_the_proof = codes(&deployed);
    for code in &refused {
        assert!(
            under_the_proof.contains(code),
            "`{code}` is not among the refusals under the proof:\n{deployed}"
        );
    }
}

/// `rooms` parsed and analyzed as a build reads it, marked as its deployment proves a person
/// to it or not.
fn analyzed(proved: bool) -> SCXMLModel {
    let mut model = sce_build::parser::SCXMLParser::new()
        .parse_string(&rooms(""), "rooms")
        .expect("rooms parses");
    sce_build::analyzer::analyze(&mut model, "rooms.scxml");
    model.principal_proved = proved;
    model
}

/// docs/adr/0116, D3, D7 and D10: the Rust generation chooses the slot from the model's
/// mark, and a `no_std` machine, which holds no person, is refused by name where a
/// deployment proves one to it. Red if the template reads the document's declaration alone,
/// or if the refusal is asked of the declaration alone.
#[test]
fn the_rust_generation_holds_the_person_where_the_model_says_one_is_proved() {
    let templates = sce_build::find_template_dir_for(Language::Rust);
    let held = sce_build::generator::generate(&analyzed(true), &templates, false)
        .expect("generated with the person's slot");
    for needle in HOLDS {
        assert!(held.contains(needle), "expected `{needle}` in:\n{held}");
    }
    let none = sce_build::generator::generate(&analyzed(false), &templates, false)
        .expect("generated with the slot of no size");
    assert!(
        none.contains(HOLDS_NONE),
        "expected `{HOLDS_NONE}` in:\n{none}"
    );

    match sce_build::generator::generate(&analyzed(true), &templates, true) {
        Err(refused) => {
            let said = refused.to_string();
            assert!(
                said.contains("no_std") && said.contains("proves to it the person"),
                "{said}"
            );
        }
        Ok(_) => panic!("a no_std machine was generated to hold a person"),
    }
}
