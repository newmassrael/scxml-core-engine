// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: the reply an invoked child sends while it starts reaches the parent before the
// host's next event. Swift AOT path.
//
// Fixture: integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AChildReplyArrivesWithoutATick {
    @Test func theChildsReplyArrivesBeforeTheHostsNextEvent() throws {
        let policy = AChildReplyArrivesWithoutATickPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        engine.processEvent(.finish)

        let seen = "hellos=\(String(describing: policy.readScriptValue("hellos"))) (wanted 1)"
        #expect(
            engine.terminalState == .done, "`finish` must carry the run to `done`. \(seen)")
        #expect(
            policy.readScriptValue("hellos") == .integer(1),
            "the child's start-time reply arrives before `finish`. \(seen)")
    }
}
