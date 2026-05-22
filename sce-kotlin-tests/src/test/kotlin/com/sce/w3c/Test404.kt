// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f5e6315f2ec211d36d839290b90cbd833e902936cc9328b605b51a480ada76bd
// generated-at: 1779411648
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test404.scxml:1
package com.sce.w3c

import com.sce.generated.test404.Test404Event
import com.sce.generated.test404.Test404State
import com.sce.generated.test404.Test404StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To execute a set of transitions, the SCXML Processor MUST first exit all the states in the transitions' exit set in exit order.
@DisplayName("Test 404 -- W3C SCXML 3.13")
class Test404 : W3CTestBase<Test404State, Test404Event>() {
    override fun createStateMachine() = Test404StateMachine()
    override val expectedPassState: Test404State = Test404State.Pass
}
