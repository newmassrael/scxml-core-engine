// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test342.Test342Event
import com.sce.generated.test342.Test342State
import com.sce.generated.test342.Test342StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST set the name field (of the _event variable) to the name of the event.
@DisplayName("Test 342 -- W3C SCXML 5.10")
class Test342 : W3CTestBase<Test342State, Test342Event>() {
    override fun createStateMachine() = Test342StateMachine(createEngine())
    override val expectedPassState: Test342State = Test342State.Pass
}
