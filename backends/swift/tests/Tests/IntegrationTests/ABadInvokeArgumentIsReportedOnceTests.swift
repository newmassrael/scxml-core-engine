// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 6.4: what each argument of an `<invoke>` costs when it cannot be read, and what
// a readable one delivers. Swift AOT path.
//
// Fixture: integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ABadInvokeArgumentIsReportedOnce {
    @Test func eachArgumentCostsWhatItsClauseSays() throws {
        let policy = ABadInvokeArgumentIsReportedOncePolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        for event in [ABadInvokeArgumentIsReportedOnceEvent.go, .finish] {
            engine.processEvent(event)
        }

        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`")
        let wrong = mismatches([
            ("errors", 5), ("started", 1), ("fromLocOk", 1), ("emptyLocLeftOut", 1),
            ("brokenLeftOut", 1),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
