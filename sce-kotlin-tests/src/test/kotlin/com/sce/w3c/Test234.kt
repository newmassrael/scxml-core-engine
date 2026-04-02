// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test234.Test234Event
import com.sce.generated.test234.Test234State
import com.sce.generated.test234.Test234StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: t MUST NOT execute the finalize handler in any other instance of invoke besides the one in the instance of invoke that created the service that generated the event.
@DisplayName("Test 234 -- W3C SCXML 6.4")
class Test234 : W3CTestBase<Test234State, Test234Event>() {
    override fun createStateMachine() = Test234StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test234State = Test234State.Pass
}
