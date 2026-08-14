// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0
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
