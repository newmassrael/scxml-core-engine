// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test372.scxml:1
package com.sce.w3c

import com.sce.generated.test372.Test372Event
import com.sce.generated.test372.Test372State
import com.sce.generated.test372.Test372StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.7: When the state machine enters the final child of a state element, the SCXML processor MUST generate the event done.state.id after completion of the onentry elements, where id is the id of the parent state.
@DisplayName("Test 372 -- W3C SCXML 3.7")
class Test372 : W3CTestBase<Test372State, Test372Event>() {
    override fun createStateMachine() = Test372StateMachine(createEngine())
    override val expectedPassState: Test372State = Test372State.Pass
}
