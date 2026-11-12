// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 under 6.4: a failed invoke param costs its pair, not the session. Swift AOT path.
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokeParamErrorStartsTheChild {
    @Test func anInvokeParamThatWillNotEvaluateCostsItsPairAndNothingElse() throws {
        let engine = Engine(policy: InvokeParamErrorStartsTheChildPolicy())
        try engine.initialize()

        // The fixture's own `timeout` is a 3 s delayed `<send>`: a channel that terminated the
        // element leaves the machine waiting on a session that was never created, and only the
        // clock turns that into a verdict.
        _ = tickUntil(engine, timeout: .seconds(10)) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)). failNoParamError: `childUp` arrived with no `error.execution` before it: the error goes on the internal queue while the `<invoke>` is being evaluated, so it is dequeued before the child's first word. failInvokeNotStarted: the child never started: one `<param>` that would not evaluate cost the whole session. failGoodParamLost: the child's `kept` did not arrive as 'here': one sibling that failed does not cost the others. failBrokenParamSeeded: the child found the empty string under `broken`: the clause says ignore the name AND the value."
        )
    }
}
