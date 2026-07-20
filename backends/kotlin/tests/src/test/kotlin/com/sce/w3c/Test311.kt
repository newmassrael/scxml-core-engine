// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
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
