// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test193.Test193Event
import com.sce.generated.test193.Test193State
import com.sce.generated.test193.Test193StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If neither the 'target' nor the 'targetexpr' attribute is specified, the SCXML Processor MUST add the event to the external event queue of the sending session.
@DisplayName("Test 193 -- W3C SCXML C.1")
class Test193 : W3CTestBase<Test193State, Test193Event>() {
    override fun createStateMachine() = Test193StateMachine()
    override val expectedPassState: Test193State = Test193State.Pass
}
