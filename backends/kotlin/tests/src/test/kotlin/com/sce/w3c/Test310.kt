// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
