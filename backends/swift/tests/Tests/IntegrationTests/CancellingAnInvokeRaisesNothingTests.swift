// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: cancelling an invoke raises no event in the invoking session. Swift AOT path.
//
// Fixture: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct CancellingAnInvokeRaisesNothing {
    @Test func leavingTheInvokingStateRaisesNoCancelEvent() throws {
        let policy = CancellingAnInvokeRaisesNothingPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        #expect(
            engine.getCurrentState() == .p, "the run has to start in `p`, with its child invoked")

        for event in [CancellingAnInvokeRaisesNothingEvent.leave, .finish] {
            engine.processEvent(event)
        }

        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`")
        #expect(
            policy.readScriptValue("spurious") == .integer(0),
            "leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session; the clause defines no such event"
        )
    }
}
