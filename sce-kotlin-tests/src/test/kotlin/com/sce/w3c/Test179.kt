// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test179.Test179Event
import com.sce.generated.test179.Test179State
import com.sce.generated.test179.Test179StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST evaluate the content element when the parent send element is evaluated and pass the resulting data unmodified to the external service when the message is delivered.
@DisplayName("Test 179 -- W3C SCXML 6.2")
class Test179 : W3CTestBase<Test179State, Test179Event>() {
    override fun createStateMachine() = Test179StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test179State = Test179State.Pass
}
