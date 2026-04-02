// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test423.Test423Event
import com.sce.generated.test423.Test423State
import com.sce.generated.test423.Test423StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Then [after invoking the new invoke handlers since the last macrostep] the Processor MUST remove events from the external event queue, waiting till events appear if necessary, until it finds one that enables a non-empty optimal transition set in the current configuration.
@DisplayName("Test 423 -- W3C SCXML 3.13")
class Test423 : W3CTestBase<Test423State, Test423Event>() {
    override fun createStateMachine() = Test423StateMachine()
    override val expectedPassState: Test423State = Test423State.Pass
}
