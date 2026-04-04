// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test174.Test174Event
import com.sce.generated.test174.Test174State
import com.sce.generated.test174.Test174StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 174 -- W3C SCXML 6.2")
class Test174 : W3CTestBase<Test174State, Test174Event>() {
    override fun createStateMachine() = Test174StateMachine(createEngine())
    override val expectedPassState: Test174State = Test174State.Pass
}
