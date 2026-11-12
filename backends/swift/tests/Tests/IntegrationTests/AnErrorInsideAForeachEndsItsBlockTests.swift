// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9 + 4.6: an error inside a `<foreach>` ends the executable block it sits in, and
// raises exactly one `error.execution`. Swift AOT path.
//
// Fixture: integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnErrorInsideAForeachEndsItsBlock {
    @Test func theErrorEndsTheBlockAndIsTheOnlyOne() throws {
        let policy = AnErrorInsideAForeachEndsItsBlockPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        for event in [AnErrorInsideAForeachEndsItsBlockEvent.go, .t, .finish] {
            engine.processEvent(event)
        }

        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`")
        let wrong = mismatches([
            ("iters1", 1), ("after1", 0), ("iters2", 1), ("after2", 0), ("iters3", 1),
            ("after3", 0), ("errors", 3), ("sent", 2),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
