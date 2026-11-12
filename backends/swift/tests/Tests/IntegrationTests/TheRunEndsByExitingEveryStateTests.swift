// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitInterpreter: the run ends by exiting every state. Swift AOT path.
//
// Fixture: integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The handlers record with `<assign>`, so this is an ECMAScript-datamodel machine.
private func started() -> (
    policy: TheRunEndsByExitingEveryStatePolicy, engine: Engine<TheRunEndsByExitingEveryStatePolicy>
) {
    let policy = TheRunEndsByExitingEveryStatePolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    #expect(engine.getCurrentState() == .inner, "the run has to start inside `inner`")
    return (policy, engine)
}

/// What the run left behind, so a failure says what happened.
private func describe(
    _ policy: TheRunEndsByExitingEveryStatePolicy, _ engine: Engine<TheRunEndsByExitingEveryStatePolicy>
) -> String {
    "active: \(engine.getActiveStates()), ended in \(String(describing: engine.terminalState)), "
        + "order=\(String(describing: policy.readScriptValue("order"))) "
        + "finalExits=\(String(describing: policy.readScriptValue("finalExits"))) "
        + "selfInFinal=\(String(describing: policy.readScriptValue("selfInFinal")))"
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct TheRunEndsByExitingEveryState {
    @Test func aRunThatReachesItsFinalExitsTheFinal() {
        let (policy, engine) = started()
        engine.processEvent(.finish)

        let seen = describe(policy, engine)
        #expect(engine.terminalState == .done, Comment(rawValue: seen))
        #expect(
            engine.getActiveStates().isEmpty,
            "exitInterpreter deletes every state it exits, the final included. \(seen)")
        #expect(
            policy.readScriptValue("finalExits") == .integer(1),
            "the final's own <onexit> must run exactly once as the run ends. \(seen)")
        #expect(
            policy.readScriptValue("selfInFinal") == .integer(1),
            "`done` must still be in the configuration during its own <onexit>. \(seen)")
        #expect(
            policy.readScriptValue("order") == .integer(12),
            "`finish` exits `inner` then `outer`. \(seen)")
    }

    @Test func aStoppedRunExitsEveryStateInnermostFirst() {
        let (policy, engine) = started()
        engine.stop()

        let seen = describe(policy, engine)
        #expect(engine.terminalState == nil, "a stopped run did not end in a final. \(seen)")
        #expect(engine.getActiveStates().isEmpty, Comment(rawValue: seen))
        #expect(
            policy.readScriptValue("order") == .integer(12),
            "stop() must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited nothing, 21 one that exited in document order instead of exit order. \(seen)"
        )
        #expect(
            policy.readScriptValue("finalExits") == .integer(0), "the run never entered `done`. \(seen)")
    }
}
