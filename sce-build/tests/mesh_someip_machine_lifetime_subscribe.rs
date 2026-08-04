// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine-lifetime subscription on SOME/IP resolves through the same
// vsomeip.json the SCXML-driven path uses.
//
// SCE_MESH.md §13's own machine-lifetime example is written with
// `transport: someip`, and the §8.1 dual-path table lists the deploy.yaml
// `subscriptions:` list beside the state-entry path without qualifying it
// per transport. The topology stage nonetheless rejected every SOME/IP
// subscription outright, so the documented deployment did not build.
//
// The reference this measures against is vsomeip + AUTOSAR SOME/IP-SD:
// a subscriber calls `request_service` / `request_event` / `subscribe`
// with ids it reads out of `vsomeip.json` by hand. Nothing in that stack
// checks the ids against the file — an eventgroup name that does not
// exist, or an event that is not a member of the group it subscribes to,
// compiles and links and then simply never delivers. SCE resolves the
// same names at build time and fails the build instead: an eventgroup
// that vsomeip.json does not declare is `mesh/external-unresolved-names`,
// an empty or multi-event group is its own diagnostic, and a subscription
// whose binding declares no eventgroup at all is
// `mesh/topology-missing-binding-field` naming the event.
//
// This file covers the resolution and emission ends. The behavioural end
// (a live subscribe reaching a live vsomeip routing manager) is the ctest
// SOME/IP suite, which needs the daemon.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

/// Subscriptions-only machine: the document carries no `<send>` at all,
/// so every transport edge in the generated router has to come from
/// deploy.yaml. A regression that silently dropped the subscription
/// would leave a router with no SOME/IP dispatch whatsoever.
const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="brake" datamodel="null" initial="armed">
  <state id="armed">
    <transition event="event.notification.vehicle_speed" target="armed"/>
  </state>
</scxml>
"##;

/// The publishing side. Present so `#chassis` resolves to a declared
/// machine; its own wiring is not what this file measures.
const CHASSIS_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="chassis" datamodel="null" initial="running">
  <state id="running">
    <transition event="event.notification.vehicle_speed" target="running"/>
  </state>
</scxml>
"##;

const VSOMEIP_JSON: &str = r##"{
  "applications": [ { "name": "brake_app" } ],
  "services": [
    {
      "name": "chassis_service",
      "service": "0x3100",
      "instance": "0x0007",
      "eventgroups": [
        { "name": "speed_group", "eventgroup": "0x0021",
          "events": ["0x8042"] }
      ]
    }
  ]
}
"##;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_someip_lifetime_{tag}"));
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

/// Build a brake machine whose only transport edge is the machine-lifetime
/// subscription. `event_declaration` is spliced into the binding so each
/// test can pick the per-event `events:` table, the binding-level flat
/// sugar, or neither.
fn compile(
    tag: &str,
    event_declaration: &str,
) -> Result<String, sce_build::mesh::error::MeshError> {
    let fx = Fixture::new(tag);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("chassis.scxml", CHASSIS_SCXML);
    fx.write("vsomeip.json", VSOMEIP_JSON);
    let deploy_path = fx.write(
        "deploy.yaml",
        &format!(
            r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#chassis":
            transport: someip
            service: chassis_service
{event_declaration}
        subscriptions:
          - event: event.notification.vehicle_speed
            source: "#chassis"
      chassis:
        source: chassis.scxml
"##
        ),
    );

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_string(BRAKE_SCXML, "brake").expect("parse");
    sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .map(|r| r.output.files[0].1.clone())
}

/// Per-event `events:` table — the spec-canonical declaration.
const PER_EVENT_TABLE: &str = r#"            events:
              "event.notification.vehicle_speed":
                event_group: speed_group
"#;

/// Binding-level flat sugar — one eventgroup for the whole binding.
const FLAT_SUGAR: &str = "            event_group: speed_group\n";

#[test]
fn per_event_declaration_emits_request_event_and_subscribe() {
    let code = compile("per_event", PER_EVENT_TABLE).expect("someip subscription must build");

    // The ids come from vsomeip.json, not from deploy.yaml: an operator
    // never writes a numeric id, and a build that invented one would be
    // the failure this whole resolution stage exists to prevent.
    assert!(
        code.contains(
            "static constexpr vsomeip::eventgroup_t \
             SOMEIP_EVENT_GROUP_CHASSIS_EVENT_NOTIFICATION_VEHICLE_SPEED = 0x0021;"
        ),
        "eventgroup id must come from vsomeip.json:\n{code}"
    );
    assert!(
        code.contains(
            "static constexpr vsomeip::event_t \
             SOMEIP_EVENT_CHASSIS_EVENT_NOTIFICATION_VEHICLE_SPEED = 0x8042;"
        ),
        "event id must come from the eventgroup's members:\n{code}"
    );

    // vsomeip needs all three calls: `request_service` opens SD discovery
    // for the (service, instance), `request_event` declares which event of
    // the group this client wants, and `subscribe` sends the SD
    // SubscribeEventgroup. Emitting any two of the three is a subscription
    // that never delivers.
    assert!(
        code.contains("request_service(\n            SOMEIP_SERVICE_CHASSIS,"),
        "request_service must be emitted for the subscription-only target:\n{code}"
    );
    assert!(
        code.contains("app.request_event("),
        "request_event must be emitted for the machine-lifetime subscription:\n{code}"
    );
    assert!(
        code.contains("app.subscribe("),
        "subscribe must be emitted for the machine-lifetime subscription:\n{code}"
    );
    assert!(
        code.contains("app.unsubscribe("),
        "the shutdown leg must be emitted too — the router unsubscribes at \
         shutdown, which is the half that distinguishes machine-lifetime \
         from a leaked subscription:\n{code}"
    );

    // A notification that arrives with no registered handler is dropped by
    // vsomeip without a trace, so the receive side has to be emitted from
    // the same declaration rather than left to the author.
    assert!(
        code.contains("register_message_handler(\n            SOMEIP_SERVICE_CHASSIS,")
            && code.contains("SOMEIP_EVENT_CHASSIS_EVENT_NOTIFICATION_VEHICLE_SPEED,"),
        "an inbound handler must be registered for the subscribed event:\n{code}"
    );

    // The init-time envelope and the dispatch arm have to agree on the
    // event name or `send_to_chassis` falls through to its unknown-event
    // return. Both sides are generated, so the pairing is checkable here.
    assert!(
        code.contains("sub_env.type = \"event.notification.vehicle_speed\";"),
        "init must dispatch the subscribe envelope under the declared event name:\n{code}"
    );
    assert!(
        code.contains("if (env.type == \"event.notification.vehicle_speed\") {"),
        "send_to_chassis must carry an arm for the subscription event:\n{code}"
    );
}

#[test]
fn binding_level_flat_sugar_resolves_the_same_ids() {
    // The flat `event_group:` sugar is the documented shorthand for a
    // binding whose events all share one group. It has to reach the
    // machine-lifetime path too, or the shorthand would silently mean
    // "SCXML sends only".
    let code = compile("flat", FLAT_SUGAR).expect("flat sugar must build");
    assert!(
        code.contains("SOMEIP_EVENT_GROUP_CHASSIS_EVENT_NOTIFICATION_VEHICLE_SPEED = 0x0021;"),
        "flat sugar must project onto the subscription event:\n{code}"
    );
    assert!(
        code.contains("SOMEIP_EVENT_CHASSIS_EVENT_NOTIFICATION_VEHICLE_SPEED = 0x8042;"),
        "flat sugar must carry the event id as well as the group id:\n{code}"
    );
}

#[test]
fn subscription_without_an_eventgroup_declaration_is_rejected() {
    // vsomeip's own failure mode here is silence: the client subscribes to
    // whatever ids the author typed and waits forever. Naming the missing
    // declaration at build time is the point of resolving by name.
    let err = compile("undeclared", "").expect_err("no eventgroup declared — must not build");
    let text = err.to_string();
    assert!(
        text.contains("event_group") && text.contains("event.notification.vehicle_speed"),
        "diagnostic must name both the missing field and the event it belongs to, got: {text}"
    );
}

#[test]
fn an_events_entry_that_only_a_subscription_uses_is_not_reported_unused() {
    // `EventBindingUnused` exists to catch a typo'd `events:` key by
    // cross-checking against what the SCXML model sends. A subscriptions-only
    // machine sends nothing, so the check has to count subscription
    // interest as a use — otherwise the correct deployment is rejected
    // with a diagnostic telling the author to delete the entry that makes
    // it work.
    let code = compile("not_unused", PER_EVENT_TABLE).expect("must not be reported unused");
    assert!(
        code.contains("SOMEIP_EVENT_GROUP_CHASSIS_EVENT_NOTIFICATION_VEHICLE_SPEED"),
        "the events: entry is used by the subscription:\n{code}"
    );
}
