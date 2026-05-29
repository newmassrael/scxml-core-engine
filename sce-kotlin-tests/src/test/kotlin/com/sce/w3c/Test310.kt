// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5af0768adc0cd444b401fc40536c0de87cadf9b1f8be7299536f4fc9ed22e337
// generated-at: 1780020098
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
