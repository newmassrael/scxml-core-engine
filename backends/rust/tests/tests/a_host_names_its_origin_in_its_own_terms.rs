// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an
// event it delivers in its own terms — Rust AOT path. The host runs `job` and
// completes it naming {`x-sce-host`, `motor`}; the document reads `motor` and
// `x-sce-host`, not an SCXML location and not the SCXML processor's type.
//
// Fixture: integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh rust-integration

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{EventOrigin, HostInvokeEvent};
use sce_rust_tests::integration::a_host_names_its_origin_in_its_own_terms::{
    AHostNamesItsOriginInItsOwnTermsPolicy as Policy,
    AHostNamesItsOriginInItsOwnTermsState as State,
};

/// The `<invoke type>` the build is told the host runs; the regeneration
/// script passes the same string as `--host-invoker`.
const HOST_TYPE: &str = "x-sce-host";

#[test]
fn a_host_names_its_origin_in_its_own_terms() {
    let script_engine: Arc<dyn sce_rust_runtime::IScriptEngine> =
        Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    // The invocation outlives the call that starts it: the host completes it
    // afterwards, under the token the start carried.
    let started: Arc<Mutex<Option<(String, u64)>>> = Arc::default();
    let recorded = Arc::clone(&started);
    e.register_invoker(HOST_TYPE, move |event: HostInvokeEvent| {
        if let HostInvokeEvent::Start(request) = event {
            *recorded.lock().expect("the start record") = Some((request.invoke_id, request.token));
        }
        None
    });
    e.initialize()
        .expect("the machine's script session is established");
    e.step();

    let (invoke_id, token) = started
        .lock()
        .expect("the start record")
        .take()
        .expect("`job` was started when `running` was entered");
    assert_eq!(invoke_id, "job");
    assert!(
        e.complete_host_invoke_from(
            HOST_TYPE,
            &invoke_id,
            token,
            "",
            Some(EventOrigin::new(HOST_TYPE, "motor")),
        ),
        "the running invocation's completion was refused"
    );
    e.step();

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    assert_eq!(
        e.policy().host_named(),
        Some(1),
        "`_event.origin` and `_event.origintype` were not the pair the host named"
    );
}
