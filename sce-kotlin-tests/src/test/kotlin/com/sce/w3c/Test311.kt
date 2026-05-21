// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test311.scxml:1
package com.sce.w3c

import com.sce.generated.test311.Test311Event
import com.sce.generated.test311.Test311State
import com.sce.generated.test311.Test311StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a location expression cannot be evaluated to yield a valid location, the SCXML processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 311 -- W3C SCXML 5.9")
class Test311 : W3CTestBase<Test311State, Test311Event>() {
    override fun createStateMachine() = Test311StateMachine(createEngine())
    override val expectedPassState: Test311State = Test311State.Pass
}
