// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
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
