// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test505.scxml:1
package com.sce.w3c

import com.sce.generated.test505.Test505Event
import com.sce.generated.test505.Test505State
import com.sce.generated.test505.Test505StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Otherwise, if the transition has 'type' "internal", its source state is a compound state and all its target states are proper descendents of its source state, the target set consists of all active states that are proper descendents of its source state.
@DisplayName("Test 505 -- W3C SCXML 3.13")
class Test505 : W3CTestBase<Test505State, Test505Event>() {
    override fun createStateMachine() = Test505StateMachine(createEngine())
    override val expectedPassState: Test505State = Test505State.Pass
}
