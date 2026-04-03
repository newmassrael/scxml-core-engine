// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test241.Test241Event
import com.sce.generated.test241.Test241State
import com.sce.generated.test241.Test241StateMachine
import com.sce.scripting.RhinoScriptEngine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST treat values specified by param and namelist identically.
@DisplayName("Test 241 -- W3C SCXML 6.4")
class Test241 : W3CTestBase<Test241State, Test241Event>() {
    override fun createStateMachine() = Test241StateMachine(RhinoScriptEngine())
    override val expectedPassState: Test241State = Test241State.Pass
    override val timeoutMs: Long = 5000L
}
