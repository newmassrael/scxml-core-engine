// GENERATED -- DO NOT EDIT (sce-codegen)
package com.sce.w3c

import com.sce.generated.test346.Test346Event
import com.sce.generated.test346.Test346State
import com.sce.generated.test346.Test346StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST place the error error.execution on the internal event queue when any attempt to change the value of a system variable is made.
@DisplayName("Test 346 -- W3C SCXML 5.10")
class Test346 : W3CTestBase<Test346State, Test346Event>() {
    override fun createStateMachine() = Test346StateMachine(createEngine())
    override val expectedPassState: Test346State = Test346State.Pass
}
