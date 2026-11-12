// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a `<send>` delay is a CSS2 time. Swift AOT path.
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ADelayIsACss2Time {
    /// On a manual clock advanced a full minute: both valid delays fire in the order their
    /// milliseconds give, and a refused message, had it been scheduled under some default wait,
    /// would have arrived and moved `bad`.
    @Test func eachDelayIsReadAsOneCss2Time() throws {
        let policy = ADelayIsACss2TimePolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        try engine.initialize()
        engine.advanceTimeMs(60000)
        engine.tick()

        #expect(engine.getCurrentState() == .done, "`a` must carry the run to `done`")
        let observed: [(name: String, want: Int64)] = [
            ("errors", 2), ("after", 0), ("bad", 0), ("aAfterB", 1),
        ]
        let wrong = observed.filter { policy.readScriptValue($0.name) != .integer($0.want) }
            .map { "\($0.name)=\(String(describing: policy.readScriptValue($0.name))) (wanted \($0.want))" }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
