// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML Conformance Test Harness for Kotlin AOT State Machines

package com.sce.w3c

import com.sce.runtime.Event
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/**
 * Abstract base class for W3C SCXML conformance tests.
 *
 * Matches C++ AOT test harness (SimpleAotTest / ScheduledAotTest):
 *   - [createStateMachine]: Factory for the generated StateMachineEngine
 *   - [expectedPassState]: The "pass" final state object
 *
 * Execution model (C++ AOT parity):
 *   - sm.initialize() — synchronous, all microsteps complete immediately
 *   - Simple tests: SM reaches final state during initialize
 *   - Scheduled tests: poll with sm.tick() for delayed sends/invokes
 *
 * Script engine selection via system property "sce.script.engine":
 *   - "rhino" (default): RhinoScriptEngine (JVM, pure Java)
 *   - "lua": LuaScriptEngine (Lua 5.4 via JNI)
 *
 * @param S State sealed interface type
 * @param E Event sealed interface type
 */
abstract class W3CTestBase<S : State, E : Event> {

    abstract fun createStateMachine(): StateMachineEngine<S, E>
    abstract val expectedPassState: S

    companion object {
        /**
         * Create a ScxmlScriptEngine based on system property "sce.script.engine".
         *
         * Both engines are compile-time dependencies of sce-kotlin-tests.
         * Generated test code calls this:
         *   override fun createStateMachine() = XxxStateMachine(createEngine())
         */
        fun createEngine(): ScxmlScriptEngine {
            val engineType = System.getProperty("sce.script.engine", "rhino").lowercase()
            return when (engineType) {
                "lua" -> LuaScriptEngine()
                "quickjs" -> QuickJSScriptEngine()
                else -> RhinoScriptEngine()
            }
        }
    }

    /**
     * Timeout for scheduled tests in milliseconds (C++ default: 2s).
     * Override for tests with delayed sends that need more time.
     */
    open val timeoutMs: Long = 2000L

    @Test
    open fun testW3CConformance() {
        val sm = createStateMachine()
        sm.initialize()

        // C++ ScheduledAotTest pattern: poll for delayed sends/invokes
        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + timeoutMs
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        try {
            assertEquals(
                expectedPassState,
                sm.currentState.value,
                "W3C conformance failed: expected Pass but reached ${sm.currentState.value}"
            )
        } finally {
            sm.cleanup()
        }
    }
}
