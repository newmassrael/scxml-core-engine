// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f746fc4eb60c4af2ce8afaa281841d74b58f08c5dc3bf4ba795e1c2351ec0f72
// generated-at: 1780577667
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test228.scxml:1
package com.sce.w3c

import com.sce.generated.test228.Test228Event
import com.sce.generated.test228.Test228State
import com.sce.generated.test228.Test228StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: the Processor MUST keep track of the unique invokeid and insure that it is included in all events that the invoked service returns to the invoking session.
@DisplayName("Test 228 -- W3C SCXML 6.4")
class Test228 : W3CTestBase<Test228State, Test228Event>() {
    override fun createStateMachine() = Test228StateMachine(createEngine())
    override val expectedPassState: Test228State = Test228State.Pass
    override val timeoutMs: Long = 5000L
}
