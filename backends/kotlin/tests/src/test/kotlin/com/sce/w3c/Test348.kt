// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
// generated-at: 0
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
