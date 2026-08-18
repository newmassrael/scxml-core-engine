// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import java.io.StringReader
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Document
import org.w3c.dom.Node
import org.xml.sax.InputSource

/**
 * W3C SCXML's ECMAScript data model appendix — the XML tree every JVM
 * script engine binds, and the shape it has.
 *
 * The three engines (Rhino, QuickJS, Lua) each expose the DOM to a
 * different language, so each owns its own binding. What they must NOT
 * each own is the *tree*: which nodes exist, what they are called and
 * what number `nodeType` reports are answers about the document, and
 * three copies of them drift. This is the one place they are given.
 *
 * The tree is the cpp reference backend's, which parses with pugixml's
 * `parse_default` — a flag set that omits `parse_ws_pcdata`,
 * `parse_comments` and `parse_pi`. `javax.xml` keeps all three, so
 * [children] drops them. While `getElementsByTagName` was the only
 * reader the difference could not be seen: that call collects elements.
 * It decides every traversal now that `childNodes` and `firstChild` are
 * readable.
 */
object SceXmlDom {
    /**
     * DOM Level 1 Core node types — the numbers `nodeType` reports.
     *
     * Four of the twelve, because four is what these trees hold: the
     * two kinds [children] drops cannot appear, and the rest —
     * attributes as nodes, entities, fragments — belong to interfaces
     * this surface does not carry. `DOCUMENT` is reported by each
     * binding's document handle rather than by a node of the tree.
     */
    const val TYPE_ELEMENT: Int = 1
    const val TYPE_TEXT: Int = 3
    const val TYPE_CDATA_SECTION: Int = 4
    const val TYPE_DOCUMENT: Int = 9

    /** The parsed document, or null when the text is not XML at all —
     *  which is W3C B.2's string reading, not an error to raise. */
    fun parse(xml: String): Document? =
        try {
            val factory = DocumentBuilderFactory.newInstance()
            factory.isNamespaceAware = true
            factory.newDocumentBuilder().parse(InputSource(StringReader(xml)))
        } catch (_: Exception) {
            null
        }

    fun nodeType(node: Node): Int =
        when (node.nodeType) {
            Node.TEXT_NODE -> TYPE_TEXT
            Node.CDATA_SECTION_NODE -> TYPE_CDATA_SECTION
            Node.DOCUMENT_NODE -> TYPE_DOCUMENT
            else -> TYPE_ELEMENT
        }

    /** The tag for an element, and DOM Level 1 Core's reserved spelling
     *  for the two character-data kinds. */
    fun nodeName(node: Node): String =
        when (nodeType(node)) {
            TYPE_TEXT -> "#text"
            TYPE_CDATA_SECTION -> "#cdata-section"
            TYPE_DOCUMENT -> "#document"
            else -> node.nodeName ?: ""
        }

    /** Whether this node kind has a nodeValue at all — character data
     *  does, an element does not (DOM Level 1 Core gives it null). */
    fun hasNodeValue(node: Node): Boolean {
        val type = nodeType(node)
        return type == TYPE_TEXT || type == TYPE_CDATA_SECTION
    }

    /** The children an author sees: document order, with the three kinds
     *  pugixml's `parse_default` never puts in a tree removed. */
    fun children(node: Node): List<Node> {
        val kept = mutableListOf<Node>()
        val childNodes = node.childNodes
        for (index in 0 until childNodes.length) {
            val child = childNodes.item(index)
            when (child.nodeType) {
                Node.COMMENT_NODE, Node.PROCESSING_INSTRUCTION_NODE -> continue
                Node.TEXT_NODE ->
                    if ((child.nodeValue ?: "").isBlank()) continue else kept.add(child)
                else -> kept.add(child)
            }
        }
        return kept
    }

    /** DOM Level 3 Core `textContent` — every descendant character-data
     *  node's content, concatenated in document order. */
    fun textContent(node: Node): String {
        if (hasNodeValue(node)) {
            return node.nodeValue ?: ""
        }
        val sb = StringBuilder()
        for (child in children(node)) {
            sb.append(textContent(child))
        }
        return sb.toString()
    }
}
