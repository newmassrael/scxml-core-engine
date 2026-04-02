// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test552.Test552Event
import com.sce.generated.test552.Test552State
import com.sce.generated.test552.Test552StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the 'src' attribute is present, the Platform MUST fetch the specified object at the time specified by the 'binding' attribute of scxml and MUST assign it as the value of the data element.
@DisplayName("Test 552 -- W3C SCXML 5.3")
class Test552 : W3CTestBase<Test552State, Test552Event>() {
    override fun createStateMachine() = Test552StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test552State = Test552State.Pass
}
