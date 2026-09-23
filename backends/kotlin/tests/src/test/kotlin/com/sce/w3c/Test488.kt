// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test488.scxml:1
package com.sce.w3c

import com.sce.generated.test488.Test488Event
import com.sce.generated.test488.Test488State
import com.sce.generated.test488.Test488StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: if the evaluation of the 'expr' produces an error, the processor MUST place the error error.execution on the internal event queue.
@DisplayName("Test 488 -- W3C SCXML 5.7")
class Test488 : W3CTestBase<Test488State, Test488Event>() {
    override fun createStateMachine() = Test488StateMachine(createEngine())
    override val expectedPassState: Test488State = Test488State.Pass
}
