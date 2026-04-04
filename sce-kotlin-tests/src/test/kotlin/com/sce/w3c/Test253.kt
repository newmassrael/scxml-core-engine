// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test253.Test253Event
import com.sce.generated.test253.Test253State
import com.sce.generated.test253.Test253StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the invoked session is of type http://www.w3.org/TR/scxml/, The SCXML Processor MUST support the use of SCXML Event/IO processor (E.1 SCXML Event I/O Processor) to communicate between the invoking and the invoked sessions.
@DisplayName("Test 253 -- W3C SCXML 6.4")
class Test253 : W3CTestBase<Test253State, Test253Event>() {
    override fun createStateMachine() = Test253StateMachine(createEngine())
    override val expectedPassState: Test253State = Test253State.Pass
}
