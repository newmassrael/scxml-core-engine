// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: the value a `srcexpr` computes selects which declared candidate runs. Swift AOT
// path.
//
//   ran `chosen`   -> `from.chosen`     -> pass
//   ran `other`    -> `from.other`      -> wrongChild
//   loaded nothing -> `error.execution` -> noChild
//   ran a stub     -> no event at all   -> parked in `probe`
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokeCandidateSelectsTheChild {
    @Test func theEvaluatedValueSelectsWhichCandidateRuns() throws {
        let engine = try initializedOnVirtualTime(InvokeCandidateSelectsTheChildPolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState())). Parking means no child spoke: a stub ran, or nothing did"
        )
        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): WrongChild means the value selected the other candidate; NoChild means nothing was loaded at all"
        )
    }
}
