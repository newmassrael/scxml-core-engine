// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test577.Test577Event
import com.sce.generated.test577.Test577State
import com.sce.generated.test577.Test577StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If neither the 'target' nor the 'targetexpr' attribute is specified, the SCXML Processor MUST add the event error.communication to the internal event queue of the sending session.
@DisplayName("Test 577 -- W3C SCXML C.2")
class Test577 : W3CTestBase<Test577State, Test577Event>() {
    override fun createStateMachine() = Test577StateMachine()
    override val expectedPassState: Test577State = Test577State.Pass
}
