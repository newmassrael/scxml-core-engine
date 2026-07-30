// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
