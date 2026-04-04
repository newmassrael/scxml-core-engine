// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test158.Test158Event
import com.sce.generated.test158.Test158State
import com.sce.generated.test158.Test158StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.9: The SCXML processor MUST execute the elements of a block of executable contentin document order.
@DisplayName("Test 158 -- W3C SCXML 4.9")
class Test158 : W3CTestBase<Test158State, Test158Event>() {
    override fun createStateMachine() = Test158StateMachine(createEngine())
    override val expectedPassState: Test158State = Test158State.Pass
}
