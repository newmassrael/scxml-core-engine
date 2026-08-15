// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The suite runs the engine it was asked for, or refuses.
//
// Every case in this suite is generated and asserts the same thing — that a
// W3C document reaches Pass. None of them can see WHICH engine it reached Pass
// on, and that is exactly what a second CI lane claims. The Kotlin arm runs
// twice, once on Rhino and once on QuickJS, and the second run is worth its
// minutes only while "-Psce.script.engine=quickjs" and "QuickJS evaluated the
// expressions" are the same statement.
//
// They were not. The selection ended in `else -> RhinoScriptEngine()`, so any
// name outside the set — a typo, a renamed engine, a property that never
// reached the JVM — produced a green run of the default. Measured before the
// repair: `./gradlew :sce-kotlin-tests:test -Psce.script.engine=nonexistent-engine`
// passed all 226 cases. A lane in that state reports the coverage it claims
// and delivers the coverage it already had.
//
// This file is what makes the claim checkable. It is not a test of the
// engines; it is a test of the sentence a lane says about itself.
//
// Nothing here writes `sce.script.engine`. The suite runs JUnit in parallel,
// so a test that set the property to observe the result was read by whichever
// sibling looked at it next — measured, as `expected: <rhino> but was:
// <nonexistent-engine>`. The mapping is asserted through `engineFor`, which
// takes the name instead of reading it.

package com.sce.w3c

import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class ScriptEngineSelectionTest {

    /**
     * The engine built is the engine named, for every name the suite admits.
     *
     * Asserted over the whole set rather than over the one this run happens to
     * use: the point is that the mapping cannot quietly collapse, and a probe
     * of a single name is green whether the others map correctly or all map to
     * the same class.
     */
    @Test
    fun every_known_engine_name_builds_that_engine() {
        val expected = mapOf(
            "rhino" to RhinoScriptEngine::class.java,
            "quickjs" to QuickJSScriptEngine::class.java,
            "lua" to LuaScriptEngine::class.java,
        )

        assertEquals(
            expected.keys,
            W3CTestBase.KNOWN_ENGINES.toSet(),
            "the names this test checks and the names the suite admits must be " +
                "the same set, or one of them is describing an engine nobody can select"
        )

        for ((name, engineClass) in expected) {
            val built = W3CTestBase.engineFor(name)
            assertEquals(
                engineClass,
                built.javaClass,
                "asking for \"$name\" must build $engineClass. Building something " +
                    "else makes every result in this suite a claim about an engine " +
                    "nobody selected."
            )
        }
    }

    /**
     * A name outside the set stops the run instead of silently defaulting.
     *
     * This is the case that was measured green before the repair, and it is
     * the one a CI lane fails on: a lane cannot notice that it asked for an
     * engine it did not get, because a suite that substituted one still
     * passes.
     */
    @Test
    fun an_unknown_engine_name_is_refused_rather_than_defaulted() {
        val failure = assertThrows(IllegalArgumentException::class.java) {
            W3CTestBase.engineFor("nonexistent-engine")
        }
        val message = failure.message ?: ""
        assertTrue(
            message.contains("nonexistent-engine"),
            "the refusal must quote the name it was given, so a lane reading " +
                "the failure sees its own typo: $message"
        )
        assertTrue(
            W3CTestBase.KNOWN_ENGINES.all { message.contains(it) },
            "and it must name what it would have accepted: $message"
        )
    }

    /**
     * Whatever this run asked for, it can be built.
     *
     * The lane-level half: the two tests above hold for every name, this one
     * holds for the name THIS invocation carries. A lane whose property never
     * reached the JVM, or carried a value the suite stopped admitting, fails
     * here rather than somewhere in the 226 cases that cannot see an engine.
     */
    @Test
    fun the_engine_this_run_asked_for_is_one_the_suite_can_build() {
        val requested = W3CTestBase.requestedEngine()
        assertTrue(
            W3CTestBase.KNOWN_ENGINES.contains(requested),
            "this run asked for \"$requested\", which the suite does not admit. " +
                "Known: ${W3CTestBase.KNOWN_ENGINES}"
        )
        // Built rather than only name-checked: the JNI-backed engines can fail
        // to load their native library, and that failure is invisible to a
        // check that compares strings.
        W3CTestBase.createEngine()
    }

    /**
     * The default is a name the suite admits.
     *
     * Making an unknown name fail is only safe while the unconfigured case
     * still has an answer. A default outside the set would turn every run that
     * asks for nothing — which is most of them — into a refusal.
     */
    @Test
    fun the_default_engine_is_one_the_suite_admits() {
        assertTrue(
            W3CTestBase.KNOWN_ENGINES.contains(W3CTestBase.DEFAULT_ENGINE),
            "the default \"${W3CTestBase.DEFAULT_ENGINE}\" is not in " +
                "${W3CTestBase.KNOWN_ENGINES}, so a run with no property set " +
                "would refuse to start"
        )
    }
}
