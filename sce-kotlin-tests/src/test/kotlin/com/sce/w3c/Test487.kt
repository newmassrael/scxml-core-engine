// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: dade9f8de6d0c296ea9dd537c4a48e14404d516e6b96273faf48e4d26f58db4f
// generated-at: 1782564443
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test487.scxml:1
package com.sce.w3c

import com.sce.generated.test487.Test487Event
import com.sce.generated.test487.Test487State
import com.sce.generated.test487.Test487StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the value specified (by 'expr' or children) is not a legal value for the location specified, the processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 487 -- W3C SCXML 5.4")
class Test487 : W3CTestBase<Test487State, Test487Event>() {
    override fun createStateMachine() = Test487StateMachine(createEngine())
    override val expectedPassState: Test487State = Test487State.Pass
}
