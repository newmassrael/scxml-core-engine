// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test411.scxml:1
package com.sce.w3c

import com.sce.generated.test411.Test411Event
import com.sce.generated.test411.Test411State
import com.sce.generated.test411.Test411StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To enter a state, the SCXML Processor MUST add the state to the active state's list. Then it MUST execute the executable content in the state's onentry handler.
@DisplayName("Test 411 -- W3C SCXML 3.13")
class Test411 : W3CTestBase<Test411State, Test411Event>() {
    override fun createStateMachine() = Test411StateMachine()
    override val expectedPassState: Test411State = Test411State.Pass
    override val timeoutMs: Long = 5000L
}
