// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test329.Test329Event
import com.sce.generated.test329.Test329State
import com.sce.generated.test329.Test329StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST cause any attempt to change the value of a system variable to fail.
@DisplayName("Test 329 -- W3C SCXML 5.10")
class Test329 : W3CTestBase<Test329State, Test329Event>() {
    override fun createStateMachine() = Test329StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test329State = Test329State.Pass
}
