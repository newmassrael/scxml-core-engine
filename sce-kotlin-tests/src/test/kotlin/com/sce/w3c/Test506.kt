// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test506.scxml:1
package com.sce.w3c

import com.sce.generated.test506.Test506Event
import com.sce.generated.test506.Test506State
import com.sce.generated.test506.Test506StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If a transition has 'type' of "internal", but its source state is not a compound state or its target states are not all proper descendents of its source state, its exit set is defined as if it had 'type' of "external".
@DisplayName("Test 506 -- W3C SCXML 3.13")
class Test506 : W3CTestBase<Test506State, Test506Event>() {
    override fun createStateMachine() = Test506StateMachine(createEngine())
    override val expectedPassState: Test506State = Test506State.Pass
}
