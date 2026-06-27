// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712
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
