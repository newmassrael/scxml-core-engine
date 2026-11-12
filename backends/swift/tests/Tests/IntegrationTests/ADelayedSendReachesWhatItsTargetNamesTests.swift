// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a `<send>`, it does not change where the send goes.
// Swift AOT path.
//
// Fixture: integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ADelayedSendReachesWhatItsTargetNames {
    @Test func aDelayedSendReachesWhatItsTargetNames() throws {
        let policy = ADelayedSendReachesWhatItsTargetNamesPolicy()
        let engine = try initializedOnVirtualTime(policy)
        _ = driveOnVirtualTime(engine, budgetMs: 3000)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState()))")
        #expect(engine.terminalState == .done, "the run must end in `done`")
        let wrong = mismatches([
            ("order", 31), ("innerInternal", 1), ("lateOk", 1), ("lateCount", 1), ("pongOk", 1),
            ("commErrors", 2), ("lostArrived", 0), ("afterStranger", 0),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
