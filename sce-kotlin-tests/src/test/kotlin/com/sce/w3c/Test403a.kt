// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f5e6315f2ec211d36d839290b90cbd833e902936cc9328b605b51a480ada76bd
// generated-at: 1779411648
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test403a.scxml:1
package com.sce.w3c

import com.sce.generated.test403a.Test403aEvent
import com.sce.generated.test403a.Test403aState
import com.sce.generated.test403a.Test403aStateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To execute a microstep, the SCXML Processor MUST execute the transitions in the corresponding optimal enabled transition set, where the optimal transition set enabled by event E in state configuration C is the largest set of transitions such that a) each transition in the set is optimally enabled by E in an atomic state in C b) no transition conflicts with another transition in the set c) there is no optimally enabled transition outside
@DisplayName("Test 403a -- W3C SCXML 3.13")
class Test403a : W3CTestBase<Test403aState, Test403aEvent>() {
    override fun createStateMachine() = Test403aStateMachine()
    override val expectedPassState: Test403aState = Test403aState.Pass
}
