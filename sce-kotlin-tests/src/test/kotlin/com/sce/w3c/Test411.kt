// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5e63a3ecc19b397697c3e24d727bc3c78cb748941f07d7f7c9d76cdea58d15a4
// generated-at: 1780032748
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
