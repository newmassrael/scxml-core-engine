// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A text past the 65,535 bytes of modified UTF-8 one JVM class-file constant
// holds arrives whole at every place the Kotlin backend writes document text as
// an expression — Kotlin AOT path.
//
// kotlinc cuts such a string into pieces that each fit, and builds it back when
// the code runs (where in the compiler is written at `kotlin_string_val`, in
// sce-build/src/filters.rs). These two machines are the evidence that it does on
// this build: that they compile is half of it, and the other half is each text
// read back from the running machine and compared with the document's own, read
// here with the JDK's XML parser rather than spelled again. A text the document
// no longer carries past one constant fails the test too, so the fixture cannot
// shrink into one that measures nothing.
//
// The constants kotlinc does NOT cut, a `const val` and an annotation argument,
// are held by sce-build/tests/a_kotlin_class_file_constant_takes_its_text_through_one_door.rs.
//
// Fixtures: backends/kotlin/tests/fixtures/jvm_long_text/jvm_long_text_static.scxml and
// backends/kotlin/tests/fixtures/jvm_long_text/jvm_long_text_ecmascript.scxml, both
// written by gen_documents.py beside them.
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh kotlin-integration

package com.sce.integration

import com.sce.integration.jvm_long_text_ecmascript.JvmLongTextEcmascriptState
import com.sce.integration.jvm_long_text_ecmascript.JvmLongTextEcmascriptStateMachine
import com.sce.integration.jvm_long_text_static.JvmLongTextStaticEvent
import com.sce.integration.jvm_long_text_static.JvmLongTextStaticStateMachine
import com.sce.runtime.Json
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.w3c.dom.Document
import org.w3c.dom.Element

@DisplayName("JvmLongText — a text past one class-file constant")
class JvmLongTextTest {

    private val scxml = "http://www.w3.org/2005/07/scxml"

    /** The fixture directory, relative to the working directory: the repository root. */
    private val fixtures = "backends/kotlin/tests/fixtures/jvm_long_text"

    /** The most bytes of modified UTF-8 one class-file constant holds. */
    private val oneConstant = 65_535

    private fun document(name: String): Document {
        val factory = DocumentBuilderFactory.newInstance()
        factory.isNamespaceAware = true
        return factory.newDocumentBuilder().parse(File("$fixtures/$name"))
    }

    /** The one element `tag` whose `attribute` is `value`. */
    private fun Document.element(tag: String, attribute: String, value: String): Element {
        val nodes = getElementsByTagNameNS(scxml, tag)
        val found = (0 until nodes.length)
            .map { nodes.item(it) as Element }
            .filter { it.getAttribute(attribute) == value }
        assertEquals(1, found.size, "the document has one <$tag $attribute=\"$value\">")
        return found.single()
    }

    /** The text of the one `<content>` under the one element `parent`. */
    private fun Document.content(parent: String): String {
        val parents = getElementsByTagNameNS(scxml, parent)
        assertEquals(1, parents.length, "the document has one <$parent>")
        val contents = (parents.item(0) as Element).getElementsByTagNameNS(scxml, "content")
        assertEquals(1, contents.length, "its <$parent> has one <content>")
        return contents.item(0).textContent
    }

    /** The text an ECMAScript string literal `'…'` at the start of `expr` spells. */
    private fun literal(expr: String): String {
        assertTrue(expr.startsWith("'"), "an expr that opens a string literal: ${expr.take(16)}")
        val close = expr.indexOf('\'', 1)
        assertTrue(close > 0, "the string literal closes")
        return expr.substring(1, close)
    }

    /** `'…' + tail`, as the text it joins. */
    private fun joined(expr: String, tail: String): String {
        assertTrue(expr.endsWith("' + tail"), "a literal joined to `tail`: …${expr.takeLast(16)}")
        return literal(expr) + tail
    }

    /** The bytes `text` takes in modified UTF-8, counted per UTF-16 unit as the class file counts them. */
    private fun modifiedUtf8(text: String): Int {
        var bytes = 0
        for (unit in text) {
            bytes += when (unit.code) {
                0 -> 2
                in 0x01..0x7f -> 1
                in 0x80..0x7ff -> 2
                else -> 3
            }
        }
        return bytes
    }

    /**
     * [actual] is the document's [expected], which is past one constant. A
     * mismatch is reported by where the two first differ and by their lengths:
     * printing two texts of 65 KB would bury the answer.
     */
    private fun assertArrived(site: String, expected: String, actual: String?) {
        val bytes = modifiedUtf8(expected)
        assertTrue(
            bytes > oneConstant,
            "$site: the document's text is $bytes bytes of modified UTF-8, which one " +
                "class-file constant holds; the fixture no longer measures anything",
        )
        if (actual == expected) return
        if (actual == null) {
            throw AssertionError("$site: the machine holds no text where the document's $bytes bytes belong")
        }
        val differs = expected.zip(actual).indexOfFirst { (want, got) -> want != got }
        val at = if (differs < 0) minOf(expected.length, actual.length) else differs
        throw AssertionError(
            "$site: the text arrived changed: ${expected.length} UTF-16 units expected, " +
                "${actual.length} held, the first difference at unit $at " +
                "(expected …${expected.substring(maxOf(0, at - 8), minOf(expected.length, at + 8))}…)",
        )
    }

    @Test
    fun aStaticMachineHoldsEachLongTextItLowers() {
        val doc = document("jvm_long_text_static.scxml")
        val tail = literal(doc.element("data", "id", "tail").getAttribute("expr"))
        val initial = literal(doc.element("data", "id", "fromData").getAttribute("expr"))
        // Past two constants, and with characters past U+FFFF: a UTF-16 pair
        // kotlinc may part between two pieces, which the build puts back together.
        assertTrue(modifiedUtf8(initial) > 2 * oneConstant, "the initial value is three pieces")
        assertTrue(initial.any { it.isHighSurrogate() }, "the static texts carry characters past U+FFFF")

        val sm = JvmLongTextStaticStateMachine()
        sm.initialize()
        try {
            assertArrived("a variable's initial value", initial, sm.fromData)
            assertArrived(
                "an <assign>'s literal",
                literal(doc.element("assign", "location", "fromAssign").getAttribute("expr")),
                sm.fromAssign,
            )
            assertArrived(
                "a literal joined to a value at run time",
                joined(doc.element("assign", "location", "fromJoin").getAttribute("expr"), tail),
                sm.fromJoin,
            )
            sm.send(JvmLongTextStaticEvent.Finish)
            sm.tick()
            assertTrue(sm.isInFinalState, "`finish` ends the machine")
            assertArrived(
                "a final's <donedata> inline content",
                doc.content("donedata"),
                Json.parse(sm.donedataAtFinal()) as? String,
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anEcmascriptMachineIsHandedEachLongTextItWrites() {
        val doc = document("jvm_long_text_ecmascript.scxml")
        val tail = literal(doc.element("data", "id", "tail").getAttribute("expr"))

        val sm = JvmLongTextEcmascriptStateMachine(W3CTestBase.createEngine())
        sm.initializeOnVirtualTime()
        try {
            sm.driveOnVirtualTime(2000L)
            assertTrue(sm.isInFinalState, "the machine never completed (parked in ${sm.currentState.value})")
            assertEquals(JvmLongTextEcmascriptState.End, sm.terminalState, "the run ends in `end`")
            assertArrived(
                "a variable's inline content",
                doc.element("data", "id", "inlineText").textContent,
                sm.fromInline(),
            )
            assertArrived(
                "a variable's expr",
                literal(doc.element("data", "id", "fromData").getAttribute("expr")),
                sm.fromData(),
            )
            assertArrived(
                "an <assign> joining a literal to a value",
                joined(doc.element("assign", "location", "fromJoin").getAttribute("expr"), tail),
                sm.fromJoin(),
            )
            assertArrived("a <send>'s inline content", doc.content("send"), sm.fromSend())
            assertArrived("a final's <donedata> inline content", doc.content("donedata"), sm.fromDone())
        } finally {
            sm.cleanup()
        }
    }
}
