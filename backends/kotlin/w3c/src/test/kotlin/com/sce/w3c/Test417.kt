// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test417.scxml:1
package com.sce.w3c

import com.sce.generated.test417.Test417Event
import com.sce.generated.test417.Test417State
import com.sce.generated.test417.Test417StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the compound state [which has the final element that we entered this microstep] is itself the child of a parallel element, and all the parallel element's other children are in final states, the Processor MUST generate the event done.state.id, where id is the id of the parallel element.
@DisplayName("Test 417 -- W3C SCXML 3.13")
class Test417 : W3CTestBase<Test417State, Test417Event>() {
    override fun createStateMachine() = Test417StateMachine()
    override val expectedPassState: Test417State = Test417State.Pass
}
