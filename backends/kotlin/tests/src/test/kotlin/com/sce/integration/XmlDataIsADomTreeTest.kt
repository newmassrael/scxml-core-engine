// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a `<data>` element's XML content is a DOM structure a
// document can walk — Kotlin AOT.
//
// The appendix obliges the Processor to create "the corresponding DOM
// structure". Measured 2026-08-18, every backend created an object carrying
// three methods — `getElementsByTagName`, `getAttribute` and a non-standard
// `getTagName`, which are the two names the W3C IRP suite reads plus one; this
// backend's Rhino engine did not even carry the third. So `doc.tagName`,
// `doc.firstChild` and `doc.childNodes.length` answered undefined with the
// whole W3C suite green.
//
// What this adds to `com.sce.ecmascript.DomReadSurfaceTest`, which measures
// the same surface against the same shared table on all three engines, is the
// SEAM: the `<data>` initializer the code generator emits, and the guards it
// lowered. A binding being right does not say a document reaches it.
//
// Fixture: integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_xml_data_is_a_dom_tree_kotlin.sh

package com.sce.integration

import com.sce.integration.xml_data_is_a_dom_tree.XmlDataIsADomTreeState
import com.sce.integration.xml_data_is_a_dom_tree.XmlDataIsADomTreeStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML B.2 — a document walks the DOM its `<data>` element declared.
@DisplayName("XmlDataIsADomTree — W3C SCXML B.2")
class XmlDataIsADomTreeTest {

    @Test
    fun aDataElementsXmlIsADomTreeTheDocumentCanWalk() {
        // The fixture reads the DOM in its guards, so this is an
        // ECMAScript-datamodel machine.
        val sm = XmlDataIsADomTreeStateMachine(W3CTestBase.createEngine())
        // Every transition is eventless, so the verdict is reached in the
        // first macrostep and no event is needed to ask the question.
        sm.initialize()

        assertNotEquals(
            XmlDataIsADomTreeState.NotADocument,
            sm.currentState.value,
            "the variable did not hold a document: nodeType === 9, nodeName === " +
                "'#document', documentElement.tagName === 'books' or hasAttribute('count') " +
                "did not hold",
        )
        assertNotEquals(
            XmlDataIsADomTreeState.WrongTree,
            sm.currentState.value,
            "the document element's children are not the two <book> elements in document " +
                "order — the whitespace between them may have become nodes, or a " +
                "sibling/parent link is missing",
        )
        assertNotEquals(
            XmlDataIsADomTreeState.NoText,
            sm.currentState.value,
            "character data did not report itself as a text node, or textContent did not " +
                "read the text below the element",
        )
        assertEquals(
            XmlDataIsADomTreeState.Settled,
            sm.currentState.value,
            "the machine reached none of its four verdicts, so the guards did not " +
                "evaluate at all",
        )
    }
}
