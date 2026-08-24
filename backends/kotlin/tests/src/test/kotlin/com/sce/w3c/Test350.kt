// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4cbf0ce468f2db0011b4fa010e6c117357964548e492f95e76a21755c70778e3
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test350.scxml:1
package com.sce.w3c

import com.sce.generated.test350.Test350Event
import com.sce.generated.test350.Test350State
import com.sce.generated.test350.Test350StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: target'. The sending SCXML Processor MUST take the value of this attribute from the 'target' attribute of the send element. The receiving SCXML Processor MUST use this value to determine which session to deliver the message to.
@DisplayName("Test 350 -- W3C SCXML C.1")
class Test350 : W3CTestBase<Test350State, Test350Event>() {
    override fun createStateMachine() = Test350StateMachine(createEngine())
    override val expectedPassState: Test350State = Test350State.Pass
}
