// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1648c68c7039bcd2d9f4b6a29e08b82f1fcf3cd79ecb3462ff4016858820460c
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test187.scxml:1
package com.sce.w3c

import com.sce.generated.test187.Test187Event
import com.sce.generated.test187.Test187State
import com.sce.generated.test187.Test187StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML session terminates before the delay interval has elapsed, the SCXML Processor MUST discard the message without attempting to deliver it.
@DisplayName("Test 187 -- W3C SCXML 6.2")
class Test187 : W3CTestBase<Test187State, Test187Event>() {
    override fun createStateMachine() = Test187StateMachine()
    override val expectedPassState: Test187State = Test187State.Pass
    override val timeoutMs: Long = 5000L
}
