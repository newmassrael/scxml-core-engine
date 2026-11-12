// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's. Swift AOT path.
//
// Fixture: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InlineContentIsReadByItsDataModel {
    @Test func inlineContentIsTheValueItsDataModelReads() throws {
        let engine = Engine(policy: InlineContentIsReadByItsDataModelPolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        let reached = engine.terminalState
        let why: String
        switch reached {
        case .failNumber:
            why =
                "`number` arrived with `_event.data` unequal to 21: under the ECMAScript data model `<content>21</content>` is the number 21, finished at build time, whether or not the sending machine needs a script engine."
        case .failText:
            why =
                "`text` arrived with `_event.data` other than 'two words': text that is no expression is the space-normalized string it spells."
        case .failObject:
            why =
                "`object` arrived without `_event.data.k === 7`: `<content>{\"k\": 7}</content>` is an object literal, the object it denotes."
        case .failDocument:
            why =
                "`done.invoke.reporter` arrived without a document whose `book` carries title=\"x\": a `<donedata>`'s XML `<content>` travels as its text and the receiver builds the DOM; it is not an expression to evaluate."
        case .pass:
            why = ""
        default:
            why =
                "settled in \(engine.getCurrentState()), which is not a verdict state: the child's events never reached the parent."
        }

        #expect(reached == .pass, Comment(rawValue: why))
    }
}
