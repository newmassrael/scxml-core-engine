// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 `<send>` `<param>` payload delivery — Rust AOT.
//
// Two paths that were fixed at the template layer with no runtime witness,
// because no committed fixture had a machine of the required shape. The
// suites could only show that nothing regressed; that same absence is why
// the defects survived as long as they did.
//
//   engine-less child -> parent   param emission used to be gated on the
//     *machine* needing a script engine rather than on the send needing
//     one, so a `datamodel="null"` child shipped its `<send>` with the
//     params dropped.
//
//   #_internal                    the internal-raise path took no event
//     data, so params were built and then discarded.
//
// The two reach distinct final states, so a failure names the path.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// (canonical, shared with the Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_send_param_payload.sh

use std::time::Duration;

use sce_rust_tests::integration::send_param_payload::{
    SendParamPayloadPolicy, SendParamPayloadState,
};

#[test]
fn send_params_reach_event_data_from_child_and_internal_queue() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = SendParamPayloadPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "send_param_payload timed out before reaching a final state — the parent \
         never saw `fromChild`, never saw its own `loopback`, or discarded a whole \
         `<send>` because one `<param>` would not evaluate (W3C SCXML 5.7.1 drops \
         the pair, not the message)"
    );

    match engine.get_current_state() {
        SendParamPayloadState::Pass => {}
        SendParamPayloadState::FailChildPayload => panic!(
            "`fromChild` arrived without `_event.data.value` — a `datamodel=\"null\"` \
             child needs no script engine, but its `<send>` still has to carry the \
             params it declares. The gate is whether this send folds to literals, \
             not whether the machine needs an engine"
        ),
        SendParamPayloadState::FailInternalPayload => panic!(
            "`loopback` arrived without `_event.data.carried` — a `<send \
             target=\"#_internal\">` must raise its params as event data, not build \
             them and drop them at the internal-raise boundary"
        ),
        SendParamPayloadState::FailNumberType => panic!(
            "`typed` arrived with `_event.data.n` unequal to 7 — `expr=\"7\"` is the \
             Number 7, and a backend that stringifies on the way through delivers \
             \"7\", which `===` finds unequal"
        ),
        SendParamPayloadState::FailStringType => panic!(
            "`typed` arrived with `_event.data.s` unequal to 'kept' — a param that \
             has to be EVALUATED reaches the runtime serialiser, whose string arm \
             must emit the value rather than an engine spelling of it"
        ),
        SendParamPayloadState::FailDuplicateParams => panic!(
            "`typed` did not carry both values of the repeated name `d` with their \
             types — W3C SCXML 6.2 lets a name repeat and every value must be \
             delivered"
        ),
        SendParamPayloadState::FailNoParamError => panic!(
            "`withBadParam` arrived with no `error.execution` before it — W3C SCXML \
             5.7.1 puts that error on the internal queue while the `<send>` is being \
             evaluated, so it is dequeued first"
        ),
        SendParamPayloadState::FailBrokenParamDelivered => panic!(
            "`_event.data.broken` arrived as the empty string — 5.7.1 says ignore \
             the name AND the value, so a receiver must find no field at all rather \
             than a placeholder under the name"
        ),
        SendParamPayloadState::FailSiblingParamLost => panic!(
            "`_event.data.kept` did not survive alongside the failed param — one \
             `<param>` that will not evaluate costs its own pair and nothing else"
        ),
        other => panic!("send_param_payload settled in {other:?}, which is not a verdict state"),
    }
}
