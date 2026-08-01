// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: ab200b8eb821f02e246ff33a9f9da5a6f5493996f3df460e1a87cc5891e5b49d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test570.scxml:1
package com.sce.w3c

import com.sce.generated.test570.Test570Event
import com.sce.generated.test570.Test570State
import com.sce.generated.test570.Test570StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.7: Immediately after generating done.state.id upon entering a final child of state, if the parent state is a child of a parallel element, and all of the parallel's other children are also in final states, the Processor MUST generate the event done.state.id where id is the id of the parallel element.
@DisplayName("Test 570 -- W3C SCXML 3.7")
class Test570 : W3CTestBase<Test570State, Test570Event>() {
    override fun createStateMachine() = Test570StateMachine(createEngine())
    override val expectedPassState: Test570State = Test570State.Pass
}
