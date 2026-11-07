// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine whose transition content, written as one method, passes the 65,535
// bytes of code one JVM method holds compiles and runs — Kotlin AOT path.
//
// Measured 2026-11-05: a large `sce-static` statechart's Kotlin machine did not
// compile, kotlinc refusing `executeTransitionContent` with
// `MethodTooLargeException`, because that one method held the content of every
// transition. The generator now writes each element's code as a method of its
// own and splits every dispatch into parts of at most 256 arms
// (tools/codegen/templates/kotlin/dispatch.kt.jinja2). That this machine
// compiles is half of the evidence; the other half is each sum its transitions
// add up read back from the running machine and compared with the document's
// own, read here with the JDK's XML parser rather than spelled again. A document
// that no longer passes one method under the old shape, or no longer has more
// states than one dispatch method holds arms, fails the test too, so the fixture
// cannot shrink into one that measures nothing.
//
// The bound on every generated method is held without a JVM by
// sce-build/tests/no_generated_kotlin_method_grows_with_the_document.rs.
//
// Fixture: backends/kotlin/method-size/fixtures/jvm_method_size_static.scxml,
// written by gen_document.py beside it.
//
// A module of its own (`:sce-kotlin-method-size`), so the 1.5 MB machine
// compiles once rather than in every compile of `:sce-kotlin-tests`
// (backends/kotlin/method-size/build.gradle.kts says why).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_jvm_method_size_kotlin.sh

package com.sce.methodsize

import com.sce.methodsize.jvm_method_size_static.JvmMethodSizeStaticEvent
import com.sce.methodsize.jvm_method_size_static.JvmMethodSizeStaticState
import com.sce.methodsize.jvm_method_size_static.JvmMethodSizeStaticStateMachine
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.w3c.dom.Element

@DisplayName("JvmMethodSize — a machine whose transition content passes one JVM method")
class JvmMethodSizeTest {

    private val scxml = "http://www.w3.org/2005/07/scxml"

    /** The fixture, relative to the working directory: the repository root. */
    private val fixture = "backends/kotlin/method-size/fixtures/jvm_method_size_static.scxml"

    /** The most bytes of code one JVM method holds. */
    private val oneMethod = 65_535

    /** The fewest bytes kotlinc writes for one checked addition of a field. */
    private val checkedAdditionBytes = 22

    /** The most arms one generated dispatch method holds. */
    private val methodItems = 256

    /** One `next` transition: the state it goes to, and what it adds to each variable. */
    private data class Next(val target: String, val adds: List<Pair<String, Long>>)

    /** Each state's `next` transition, by the state's id, read from the document. */
    private fun nexts(): Map<String, Next> {
        val factory = DocumentBuilderFactory.newInstance()
        factory.isNamespaceAware = true
        val doc = factory.newDocumentBuilder().parse(File(fixture))
        val states = doc.getElementsByTagNameNS(scxml, "state")
        val found = LinkedHashMap<String, Next>()
        for (i in 0 until states.length) {
            val state = states.item(i) as Element
            val next = state.children("transition").singleOrNull { it.getAttribute("event") == "next" } ?: continue
            val adds = next.children("assign").map { assign ->
                val location = assign.getAttribute("location")
                val (read, amount) = assign.getAttribute("expr").split(" + ")
                assertEquals(location, read, "each assignment adds to the variable it writes")
                location to amount.toLong()
            }
            found[state.getAttribute("id")] = Next(next.getAttribute("target"), adds)
        }
        return found
    }

    /** The child elements of this element named `tag`, in document order. */
    private fun Element.children(tag: String): List<Element> =
        (0 until childNodes.length)
            .map { childNodes.item(it) }
            .filterIsInstance<Element>()
            .filter { it.localName == tag }

    /** What each published variable holds, by its id. */
    private fun held(sm: JvmMethodSizeStaticStateMachine): Map<String, Long> = mapOf(
        "steps" to sm.steps.toLong(),
        "entered" to sm.entered.toLong(),
        "left" to sm.left.toLong(),
        "a0" to sm.a0.toLong(),
        "a1" to sm.a1.toLong(),
        "a2" to sm.a2.toLong(),
        "a3" to sm.a3.toLong(),
        "a4" to sm.a4.toLong(),
        "a5" to sm.a5.toLong(),
        "a6" to sm.a6.toLong(),
        "a7" to sm.a7.toLong(),
        "a8" to sm.a8.toLong(),
        "a9" to sm.a9.toLong(),
        "a10" to sm.a10.toLong(),
    )

    @Test
    fun theDocumentStillPassesOneMethodUnderTheOldShape() {
        val nexts = nexts()
        assertTrue(
            nexts.size > methodItems,
            "the document has ${nexts.size} states with a transition, which one dispatch method " +
                "holds; the fixture no longer splits a dispatch",
        )
        val additions = nexts.values.sumOf { it.adds.size }
        assertTrue(
            additions * checkedAdditionBytes > oneMethod,
            "the document's transition content is $additions checked additions, which one " +
                "method held; the fixture no longer measures anything",
        )
    }

    @Test
    fun everyTransitionRunsItsOwnContentInOrder() {
        val nexts = nexts()
        val rounds = nexts.size + 3
        val expected = HashMap<String, Long>()
        var at = "s0"
        repeat(rounds) {
            val next = nexts.getValue(at)
            for ((variable, amount) in next.adds) {
                expected[variable] = (expected[variable] ?: 0L) + amount
            }
            at = next.target
        }
        // `s0` is entered once when the machine starts, and every `next` leaves one
        // state and enters another.
        expected["entered"] = rounds + 1L
        expected["left"] = rounds.toLong()

        val sm = JvmMethodSizeStaticStateMachine()
        sm.initialize()
        try {
            repeat(rounds) {
                sm.send(JvmMethodSizeStaticEvent.Next)
                sm.tick()
            }
            val held = held(sm)
            for ((variable, sum) in expected) {
                assertEquals(sum, held[variable], "what `$variable` holds after $rounds transitions")
            }
            assertTrue(
                JvmMethodSizeStaticState.S3 in sm.snapshot.value.configuration,
                "after $rounds transitions of ${nexts.size} the machine stands in s3: ${sm.snapshot.value}",
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theDeepHistoryReturnsToTheStateLeft() {
        val sm = JvmMethodSizeStaticStateMachine()
        sm.initialize()
        try {
            repeat(5) {
                sm.send(JvmMethodSizeStaticEvent.Next)
                sm.tick()
            }
            sm.send(JvmMethodSizeStaticEvent.Pause)
            sm.tick()
            assertEquals(setOf(JvmMethodSizeStaticState.Paused), sm.snapshot.value.configuration)
            sm.send(JvmMethodSizeStaticEvent.Resume)
            sm.tick()
            assertEquals(
                setOf(JvmMethodSizeStaticState.All, JvmMethodSizeStaticState.S5),
                sm.snapshot.value.configuration,
                "the deep history `back` restores s5, the state `pause` left",
            )
            // Started in s0, five `next`s and the return through the history.
            assertEquals(7L, sm.entered.toLong(), "each entry of an `s` state ran its <onentry>")
            assertEquals(6L, sm.left.toLong(), "each exit of an `s` state ran its <onexit>")
        } finally {
            sm.cleanup()
        }
    }
}
