// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test421.Test421Event
import com.sce.generated.test421.Test421State
import com.sce.generated.test421.Test421StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the set (of eventless transitions) is empty, the Processor MUST remove events from the internal event queue until the queue is empty or it finds an event that enables a non-empty optimal transition set in the current configuration.
@DisplayName("Test 421 -- W3C SCXML 3.13")
class Test421 : W3CTestBase<Test421State, Test421Event>() {
    override fun createStateMachine() = Test421StateMachine()
    override val expectedPassState: Test421State = Test421State.Pass
}
