// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test311.Test311Event
import com.sce.generated.test311.Test311State
import com.sce.generated.test311.Test311StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a location expression cannot be evaluated to yield a valid location, the SCXML processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 311 -- W3C SCXML 5.9")
class Test311 : W3CTestBase<Test311State, Test311Event>() {
    override fun createStateMachine() = Test311StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test311State = Test311State.Pass
}
