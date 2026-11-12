// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 4.9: a `<send>` argument that cannot be evaluated discards its message. Swift AOT
// path.
//
// Fixture: integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ABadSendArgumentDiscardsItsMessage {
    /// On a manual clock advanced a full minute before `finish`: a channel that scheduled the
    /// message whose `delayexpr` failed, under some default delay, delivers it within that minute
    /// and moves `sent`.
    @Test func eachBadArgumentDiscardsItsMessage() throws {
        let policy = ABadSendArgumentDiscardsItsMessagePolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        try engine.initialize()
        engine.advanceTimeMs(60000)
        engine.processEvent(.finish)

        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`")
        let wrong = mismatches([("errors", 6), ("sent", 0), ("after", 0)]) {
            policy.readScriptValue($0)
        }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
