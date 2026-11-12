// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a document walks the DOM its `<data>` element declared. Swift AOT path.
//
// Fixture: integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct XmlDataIsADomTree {
    @Test func aDataElementsXmlIsADomTreeTheDocumentCanWalk() throws {
        // The fixture reads the DOM in its guards, so this is an ECMAScript-datamodel machine.
        let engine = Engine(policy: XmlDataIsADomTreePolicy())
        // Every transition is eventless, so the verdict is reached in the first macrostep and no
        // event is needed to ask the question.
        try engine.initialize()

        #expect(
            engine.getCurrentState() != .notADocument,
            "the variable did not hold a document: nodeType === 9, nodeName === '#document', documentElement.tagName === 'books' or hasAttribute('count') did not hold"
        )
        #expect(
            engine.getCurrentState() != .wrongTree,
            "the document element's children are not the two <book> elements in document order: the whitespace between them may have become nodes, or a sibling/parent link is missing"
        )
        #expect(
            engine.getCurrentState() != .noText,
            "character data did not report itself as a text node, or textContent did not read the text below the element"
        )
        #expect(
            engine.getCurrentState() == .settled,
            "the machine reached none of its four verdicts, so the guards did not evaluate at all")
    }
}
