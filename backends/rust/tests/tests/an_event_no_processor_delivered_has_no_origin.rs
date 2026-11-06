// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
// delivered has no origin — Rust AOT path. The host raises `posted` naming no
// party, and its processor answers `host.ask` with `host.reply` naming none;
// the document finds `_event.origin` and `_event.origintype` blank for both,
// which the ECMAScript data model reads as undefined (nil in the Lua it lowers
// to).
//
// Fixture: integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_event_no_processor_delivered_has_no_origin.sh

use sce_rust_runtime::{HostSendRequest, HostSendResponse};
use sce_rust_tests::integration::an_event_no_processor_delivered_has_no_origin::{
    AnEventNoProcessorDeliveredHasNoOriginEvent as Event,
    AnEventNoProcessorDeliveredHasNoOriginPolicy as Policy,
    AnEventNoProcessorDeliveredHasNoOriginState as State,
};

/// The `<send type>` the build is told the host serves; the regeneration
/// script passes the same string as `--host-processor`.
const HOST_TYPE: &str = "x-sce-host";

#[test]
fn an_event_no_processor_delivered_has_no_origin() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.register_event_processor(HOST_TYPE, |request: HostSendRequest| {
        assert_eq!(request.event_name, "host.ask");
        vec![HostSendResponse {
            event_name: "host.reply".to_string(),
            event_data: String::new(),
            origin: None,
        }]
    });
    e.initialize();
    e.step();

    e.raise_external(Event::Posted, "");
    e.step();

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    let p = e.policy();
    let observed = [
        ("postedBlank", p.posted_blank(), 1),
        ("replyBlank", p.reply_blank(), 1),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
