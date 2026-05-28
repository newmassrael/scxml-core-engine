// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test335.scxml:1
package com.sce.w3c

import com.sce.generated.test335.Test335Event
import com.sce.generated.test335.Test335State
import com.sce.generated.test335.Test335StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If an event was not received from an external entity, the Processor MUST leave the origin field blank.
@DisplayName("Test 335 -- W3C SCXML 5.10")
class Test335 : W3CTestBase<Test335State, Test335Event>() {
    override fun createStateMachine() = Test335StateMachine()
    override val expectedPassState: Test335State = Test335State.Pass
}
