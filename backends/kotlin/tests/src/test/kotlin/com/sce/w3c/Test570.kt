// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
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
