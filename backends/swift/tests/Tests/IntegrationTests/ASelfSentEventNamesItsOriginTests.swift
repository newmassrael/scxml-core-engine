// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends itself names its origin. Swift AOT path.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ASelfSentEventNamesItsOrigin {
    @Test func aSelfSentEventNamesItsOrigin() throws {
        let policy = ASelfSentEventNamesItsOriginPolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        try engine.initialize()
        engine.advanceTimeMs(1000)
        engine.tick()

        #expect(engine.getCurrentState() == .done, "the run must end in `done`")
        let observed: [(name: String, want: Int64)] = [
            ("immediateOk", 1), ("replied", 1), ("delayedOk", 1), ("unreachable", 1), ("strayed", 0),
        ]
        let wrong = observed.filter { policy.readScriptValue($0.name) != .integer($0.want) }
            .map { "\($0.name)=\(String(describing: policy.readScriptValue($0.name))) (wanted \($0.want))" }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
