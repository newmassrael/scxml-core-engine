// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test298.Test298Event
import com.sce.generated.test298.Test298State
import com.sce.generated.test298.Test298StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: If the 'location' attribute on a param element does not refer to a valid location in the data model, the processor MUST place the error error.execution on the internal event queue.
@DisplayName("Test 298 -- W3C SCXML 5.7")
class Test298 : W3CTestBase<Test298State, Test298Event>() {
    override fun createStateMachine() = Test298StateMachine(createEngine())
    override val expectedPassState: Test298State = Test298State.Pass
}
