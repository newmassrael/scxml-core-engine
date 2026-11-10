// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the
// SCXML Event I/O Processor delivers is the address its sender published, on
// every route a session sends by, and a reply to it arrives — Rust AOT path.
//
// Fixture: integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh rust-integration

use std::time::Duration;

use sce_rust_tests::integration::an_origin_is_the_address_its_sender_published::{
    AnOriginIsTheAddressItsSenderPublishedPolicy as Policy,
    AnOriginIsTheAddressItsSenderPublishedState as State,
};

/// A self-send, a child's send to `#_parent` and a parent's send to
/// `#_kid`: each origin is the sender's published location with the SCXML
/// processor's URI, and each reply sent to it arrives.
#[test]
fn an_origin_is_the_address_its_sender_published() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize()
        .expect("the machine's script session is established");

    let completed = e.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "the run never reached a final state: a reply sent to an origin reached nobody"
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    let p = e.policy();
    let observed = [
        ("selfOk", p.self_ok(), 1),
        ("selfReplied", p.self_replied(), 1),
        ("childOk", p.child_ok(), 1),
        ("childReplied", p.child_replied(), 1),
        ("parentOk", p.parent_ok(), 1),
        ("parentReplied", p.parent_replied(), 1),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
