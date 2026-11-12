// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: every event an invoked child sends to `#_parent` reaches the parent, however many
// it sends in one tick. Swift AOT path.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AChildMaySendManyEventsInOneTick {
    @Test func everyEventTheChildSentArrives() throws {
        let policy = AChildMaySendManyEventsInOneTickPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        engine.processEvent(.finish)

        let seen = "ticks=\(String(describing: policy.readScriptValue("ticks"))) (wanted 110)"
        #expect(
            engine.terminalState == .done, "`finish` must carry the run to `done`. \(seen)")
        #expect(
            policy.readScriptValue("ticks") == .integer(110),
            "every event the child sent arrives before `finish`. \(seen)")
    }
}
