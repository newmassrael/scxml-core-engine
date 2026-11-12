// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 4.9: a `<param>` that cannot be read ends its block, and the message goes
// without it. Swift AOT path.
//
// Fixture: integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ABadSendParamEndsItsBlock {
    @Test func theMessageGoesAndTheBlockStops() throws {
        let policy = ABadSendParamEndsItsBlockPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        engine.processEvent(.finish)

        // errors: each unreadable <param> raises one error.execution; partials: both internal sends
        // go, carrying the good pair without the bad one; bares: the external send goes with its
        // empty pair left out; after: the <param> error ends the block, so nothing after the <send>
        // runs; carried: a valid location param is sent with its value.
        let wrong = mismatches([
            ("errors", 3), ("partials", 2), ("bares", 1), ("after", 0), ("carried", 1),
        ]) { policy.readScriptValue($0) }
        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`. \(wrong)")
        #expect(wrong.isEmpty, "observed: \(wrong) (wanted errors 3 / partials 2 / bares 1 / after 0 / carried 1)")
    }
}
