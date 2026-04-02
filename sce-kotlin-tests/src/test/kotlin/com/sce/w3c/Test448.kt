// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test448.Test448Event
import com.sce.generated.test448.Test448State
import com.sce.generated.test448.Test448StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must place all variables in a single global ECMAScript scope.
@DisplayName("Test 448 -- W3C SCXML B.2")
class Test448 : W3CTestBase<Test448State, Test448Event>() {
    override fun createStateMachine() = Test448StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test448State = Test448State.Pass
}
