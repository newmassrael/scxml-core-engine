// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test553.scxml:1
package com.sce.w3c

import com.sce.generated.test553.Test553Event
import com.sce.generated.test553.Test553State
import com.sce.generated.test553.Test553StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the evaluation of send's arguments produces an error, If the evaluation of send's arguments produces an error, the Processor MUST discard the message without attempting to deliver it.
@DisplayName("Test 553 -- W3C SCXML 6.2")
class Test553 : W3CTestBase<Test553State, Test553Event>() {
    override fun createStateMachine() = Test553StateMachine(createEngine())
    override val expectedPassState: Test553State = Test553State.Pass
    override val timeoutMs: Long = 5000L
}
