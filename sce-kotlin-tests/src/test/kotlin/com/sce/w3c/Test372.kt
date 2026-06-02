// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d2f0bcf4d5c727ad2446a904193402929b9b2d65dfec5e5c07ad3bc881483b09
// generated-at: 1780358475
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
