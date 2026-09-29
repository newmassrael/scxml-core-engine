// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-9.5
//
// SCE_MESH §9.5: a target named at run time is a choice among the
// already-declared bindings. `mesh_srcexpr_reaches_declared_bindings.rs`
// pins that for a mesh-rpc `srcexpr`; this file pins it for a
// `<send targetexpr>`, which the C++ generated router must be just as ready
// for — §scxml-6.2.4 routes a `targetexpr` as the same value written in
// `target`.
//
// ⚠ WHAT THIS PINS. The router's routes and its `resolvePattern` table were
// built from written `target`s alone. A document whose reply to a peer was a
// `targetexpr` got no `<machine>_transport.h` at all, and one that happened to
// write the same peer elsewhere got a route but classified the reply as
// FireForget, so the requester's correlation never retired it
// (measured 2026-09-29).

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_targetexpr_bindings_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        Self { dir }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.join(name);
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(content.as_bytes()).expect("write");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A responder whose only peer send is a reply named by `targetexpr`; `event`
/// is the `<send>`'s event attribute, or its `eventexpr` when `by_expr`.
fn targetexpr_only_responder(event: &str, by_expr: bool) -> String {
    let event_attr = if by_expr {
        format!("eventexpr=\"'{event}'\"")
    } else {
        format!("event=\"{event}\"")
    };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" name="responder" initial="ready">
  <state id="ready">
    <transition event="service.request.ping" target="replying"/>
  </state>
  <state id="replying">
    <onentry>
      <send targetexpr="'#alpha'" {event_attr}/>
    </onentry>
  </state>
</scxml>
"##
    )
}

/// A requester the responder answers.
fn requester(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" name="{name}" initial="calling">
  <state id="calling">
    <invoke type="sce:mesh-rpc" src="#responder">
      <param name="_mesh_event" expr="'service.request.ping'"/>
    </invoke>
    <transition event="done.invoke" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##
    )
}

fn deploy(bindings: &[&str]) -> String {
    let mut responder_bindings = String::new();
    let mut machines = String::new();
    for name in bindings {
        responder_bindings.push_str(&format!("          \"#{name}\": {{ transport: local }}\n"));
        machines.push_str(&format!(
            "      {name}:\n        source: {name}.scxml\n        bindings:\n          \"#responder\": {{ transport: local }}\n"
        ));
    }
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      responder:
        source: responder.scxml
        bindings:
{responder_bindings}{machines}"##
    )
}

fn parse(src: &str, name: &str) -> sce_build::model::SCXMLModel {
    let mut parser = sce_build::parser::SCXMLParser::new();
    parser.parse_string(src, name).expect("parse")
}

/// The responder's generated transport, which must exist.
fn transport_for(responder: &str, bindings: &[&str], tag: &str) -> (String, Vec<String>) {
    let fx = Fixture::new(tag);
    fx.write("responder.scxml", responder);
    for name in bindings {
        fx.write(&format!("{name}.scxml"), &requester(name));
    }
    let deploy_path = fx.write("deploy.yaml", &deploy(bindings));

    let mut model = parse(responder, "responder");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    let warnings: Vec<String> = result
        .dynamic_target_sends
        .iter()
        .filter(|s| s.is_warning())
        .map(ToString::to_string)
        .collect();
    let transport = result
        .output
        .files
        .iter()
        .find(|(name, _)| name == "responder_transport.h")
        .map(|(_, code)| code.clone())
        .unwrap_or_else(|| {
            panic!(
                "no transport emitted for a document whose only peer send is a targetexpr; \
                 files: {:?}",
                result
                    .output
                    .files
                    .iter()
                    .map(|(n, _)| n)
                    .collect::<Vec<_>>()
            )
        });
    (transport, warnings)
}

/// Every declared binding gets its route, and the reply is classified as
/// the reply it is.
#[test]
fn a_targetexpr_reply_routes_to_every_declared_binding_as_a_reply() {
    let (transport, warnings) = transport_for(
        &targetexpr_only_responder("service.response.ping", false),
        &["alpha", "beta"],
        "reply",
    );

    // Both bindings, not only the one this expression happens to name.
    for name in ["alpha", "beta"] {
        assert!(
            transport.contains(&format!("if (target == \"#{name}\")")),
            "the router has no route to the declared binding '#{name}'"
        );
    }
    // `resolvePattern` classifies the reply by its name: RpcReply (wire 3),
    // not the FireForget fallback an unlisted event gets.
    // The definition, not the first call site.
    let table = transport
        .split("resolvePattern(const std::string")
        .nth(1)
        .expect("resolvePattern is defined");
    let branch = table
        .split("eventName == \"service.response.ping\"")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .unwrap_or_else(|| panic!("resolvePattern has no branch for the reply:\n{table}"));
    assert!(
        branch.contains("static_cast<PK>(3)"),
        "service.response.ping is not classified as RpcReply (3): {branch}"
    );
    assert!(
        warnings.is_empty(),
        "nothing is lost, nothing is warned: {warnings:?}"
    );
}

/// An `eventexpr` still gives every binding its route; only its pattern is
/// unknown, and that is what the build says.
#[test]
fn an_eventexpr_targetexpr_send_still_routes_and_warns_of_the_pattern() {
    let (transport, warnings) = transport_for(
        &targetexpr_only_responder("service.response.ping", true),
        &["alpha"],
        "eventexpr",
    );

    assert!(
        transport.contains("if (target == \"#alpha\")"),
        "an eventexpr must not cost the send its route"
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].contains("FireForget"),
        "the warning names what is lost: {}",
        warnings[0]
    );
}
