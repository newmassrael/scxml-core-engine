// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test411.Test411Event
import com.sce.generated.test411.Test411State
import com.sce.generated.test411.Test411StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To enter a state, the SCXML Processor MUST add the state to the active state's list. Then it MUST execute the executable content in the state's onentry handler.
@DisplayName("Test 411 -- W3C SCXML 3.13")
class Test411 : W3CTestBase<Test411State, Test411Event>() {
    override fun createStateMachine() = Test411StateMachine()
    override val expectedPassState: Test411State = Test411State.Pass
}
