// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4, docs/adr/0098: a value an invoke hands its child is the initial value of the child's
// top-level `<data>` of that name, declared in place of its own. Swift AOT path.
//
// The parent judges what each child reports and lands a failure in a `<final>` named for the
// sentence that broke. The children started alone, handed their values by `acceptSeeds`, are
// EcmaScriptMachinesTests/AChildTakesAHandedValueInPlaceOfItsDataTests.swift.
//
// Fixture: integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/materialize.sh swift-integration

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AChildTakesAHandedValueInPlaceOfItsData {
    @Test func aChildTakesEachHandedValueInPlaceOfItsTopLevelData() throws {
        let engine = Engine(policy: AChildTakesAHandedValueInPlaceOfItsDataPolicy())
        try engine.initialize()

        // Four sequential invokes, each answering in its own macrostep.
        _ = tickUntil(engine, timeout: .seconds(5)) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)). failOwnValueRan / failChildRaised: the own expression of a <data> whose value was handed was evaluated. failHandedValueLost / failNamelistValueLost / failNamelistOwnValueRaised: a handed value did not take its <data>'s place. failStateDataTookTheValue / failLateStateDataTookTheValue: a state's <data>, early- or late-bound, took a handed value. failUnmatchedNameAdded / failNamelistNameAdded: a name no top-level <data> declares became a variable. failObjectNotWhole: an object lost its structure. failChildSharedTheParentsObject: the child was handed the parent's object, not a copy. failScriptReadItsOwnValue / failEntryReadItsOwnValue: the top-level <script> or the initial <onentry> ran before the values were declared. failParentRaised: the parent could not evaluate what it hands. nil: a child never reported, or the child of firstMacrostep read its own value in its first macrostep."
        )
    }
}
