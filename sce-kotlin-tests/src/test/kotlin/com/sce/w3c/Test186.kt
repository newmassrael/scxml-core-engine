// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test186.Test186Event
import com.sce.generated.test186.Test186State
import com.sce.generated.test186.Test186StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The Processor MUST evaluate all arguments to send when the send element is evaluated, and not when the message is actually dispatched.
@DisplayName("Test 186 -- W3C SCXML 6.2")
class Test186 : W3CTestBase<Test186State, Test186Event>() {
    override fun createStateMachine() = Test186StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test186State = Test186State.Pass
    override val timeoutMs: Long = 10_000L
}
