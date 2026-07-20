// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
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
