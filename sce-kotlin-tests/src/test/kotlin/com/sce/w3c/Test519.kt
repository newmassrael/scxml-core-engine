// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test519.Test519Event
import com.sce.generated.test519.Test519State
import com.sce.generated.test519.Test519StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If one or more param children are present [in send], the SCXML Processor MUST map their names (i.e. name attributes) and values to HTTP POST parameters
@DisplayName("Test 519 -- W3C SCXML C.2")
class Test519 : W3CTestBase<Test519State, Test519Event>() {
    override fun createStateMachine() = Test519StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test519State = Test519State.Pass
}
