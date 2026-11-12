// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: an invoke `<param>` is the invoking session's value. Swift AOT path.
//
// Fixture: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokeParamSeedsDeclaredChildData {
    @Test func anInvokeParamCarriesTheInvokingSessionsValue() throws {
        let engine = Engine(policy: InvokeParamSeedsDeclaredChildDataPolicy())
        try engine.initialize()

        // Three sequential invokes, each answering in its own macrostep.
        _ = tickUntil(engine, timeout: .seconds(5)) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)). failChildEvaluatedTheExpression: the child evaluated the author's `<param expr>` text in its own data model and found its own `token`: the clause says the value of the param element, which only the invoking session can produce. failParentOnlyExprLost: the expression named a variable only the parent has and nothing arrived. failUnmatchedParamEnteredTheChild: a `<param>` naming no top-level `<data>` of the child became a variable there anyway: the clause forbids adding it. failShadowSeedLost / failDeclaredParamLost / failNamelistValueLost: the child saw neither the parent's value nor a shadow, so its own `<data>` default stood: nothing was seeded at all."
        )
    }
}
