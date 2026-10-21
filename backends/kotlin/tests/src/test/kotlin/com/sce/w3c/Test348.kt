// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test348.scxml:1
package com.sce.w3c

import com.sce.generated.test348.Test348Event
import com.sce.generated.test348.Test348State
import com.sce.generated.test348.Test348StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: name'. The sending SCXML Processor MUST take the value of this attribute from the 'event' attribute of the send element. The receiving SCXML Processor MUST use it as the value the 'name' field in the event that it generates.
@DisplayName("Test 348 -- W3C SCXML C.1")
class Test348 : W3CTestBase<Test348State, Test348Event>() {
    override fun createStateMachine() = Test348StateMachine()
    override val expectedPassState: Test348State = Test348State.Pass
}
