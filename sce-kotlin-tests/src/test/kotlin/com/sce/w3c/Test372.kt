// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test372.Test372Event
import com.sce.generated.test372.Test372State
import com.sce.generated.test372.Test372StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.7: When the state machine enters the final child of a state element, the SCXML processor MUST generate the event done.state.id after completion of the onentry elements, where id is the id of the parent state.
@DisplayName("Test 372 -- W3C SCXML 3.7")
class Test372 : W3CTestBase<Test372State, Test372Event>() {
    override fun createStateMachine() = Test372StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test372State = Test372State.Pass
}
