// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test323.Test323Event
import com.sce.generated.test323.Test323State
import com.sce.generated.test323.Test323StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _name at load time to the value of the 'name' attribute of the scxml element. 	a
@DisplayName("Test 323 -- W3C SCXML 5.10")
class Test323 : W3CTestBase<Test323State, Test323Event>() {
    override fun createStateMachine() = Test323StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test323State = Test323State.Pass
}
