// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.6.2: the value of a `<send>`'s `<content expr>` is its payload. Swift AOT path.
//
// Fixture: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ASendContentExprIsThePayload {
    @Test func aSendContentExprIsThePayload() throws {
        let policy = ASendContentExprIsThePayloadPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState()))")
        #expect(engine.terminalState == .done, "the run must end in `done`")
        let wrong = mismatches([
            ("numberOk", 1), ("objectOk", 1), ("textOk", 1), ("errors", 1), ("badArrived", 1),
            ("badEmpty", 1), ("afterBad", 0),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
