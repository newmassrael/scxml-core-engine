// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test312.Test312Event
import com.sce.generated.test312.Test312State
import com.sce.generated.test312.Test312StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a value expression does not return a legal data value, the SCXML processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 312 -- W3C SCXML 5.9")
class Test312 : W3CTestBase<Test312State, Test312Event>() {
    override fun createStateMachine() = Test312StateMachine(createEngine())
    override val expectedPassState: Test312State = Test312State.Pass
}
