// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: eef83a0380a6f32e69bd8e491d75a942150e8193a11c5aedb68d2fc11fa47b6e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test310.scxml:1
package com.sce.w3c

import com.sce.generated.test310.Test310Event
import com.sce.generated.test310.Test310State
import com.sce.generated.test310.Test310StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: All datamodels MUST support the 'In()' predicate, which takes a stateID as its argument and returns true if the state machine is in that state.
@DisplayName("Test 310 -- W3C SCXML 5.9")
class Test310 : W3CTestBase<Test310State, Test310Event>() {
    override fun createStateMachine() = Test310StateMachine()
    override val expectedPassState: Test310State = Test310State.Pass
}
