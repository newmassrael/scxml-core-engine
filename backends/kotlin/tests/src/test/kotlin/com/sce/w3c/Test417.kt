// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
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
