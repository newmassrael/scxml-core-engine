// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c96808b03e7b119d29792dbf258f9125c91be8c72d4823c8f9b56e0e05a3240b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test186.scxml:1
package com.sce.w3c

import com.sce.generated.test186.Test186Event
import com.sce.generated.test186.Test186State
import com.sce.generated.test186.Test186StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The Processor MUST evaluate all arguments to send when the send element is evaluated, and not when the message is actually dispatched.
@DisplayName("Test 186 -- W3C SCXML 6.2")
class Test186 : W3CTestBase<Test186State, Test186Event>() {
    override fun createStateMachine() = Test186StateMachine(createEngine())
    override val expectedPassState: Test186State = Test186State.Pass
    override val timeoutMs: Long = 5000L
}
