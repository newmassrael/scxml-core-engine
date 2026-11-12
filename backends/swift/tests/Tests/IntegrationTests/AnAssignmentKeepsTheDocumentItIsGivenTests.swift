// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves `v` holding the DOM a
// `<send>`'s XML `<content>` arrived as, and so does a member-path location. Swift AOT path.
//
// Fixture: integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnAssignmentKeepsTheDocumentItIsGiven {
    @Test func bothLocationsHoldTheDocumentTheEventCarried() throws {
        let engine = Engine(policy: AnAssignmentKeepsTheDocumentItIsGivenPolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        let reached = engine.terminalState
        let why: String
        switch reached {
        case .nameLost:
            why =
                "`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the document in `v`: the value was read out of the data model and written back."
        case .memberLost:
            why =
                "`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not leave the document in `holder.doc`."
        case .kept:
            why = ""
        default:
            why =
                "settled in \(engine.getCurrentState()), which is not a verdict state: its own `doc` event never arrived."
        }

        #expect(reached == .kept, Comment(rawValue: why))
    }
}
