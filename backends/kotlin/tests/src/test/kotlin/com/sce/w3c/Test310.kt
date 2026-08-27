// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0afb1f0f0230f40c373aa80a890f61f2cc90b35724e7d86493a9e44e197b2d1b
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
