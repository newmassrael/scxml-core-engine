// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1648c68c7039bcd2d9f4b6a29e08b82f1fcf3cd79ecb3462ff4016858820460c
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test207.scxml:1
package com.sce.w3c

import com.sce.generated.test207.Test207Event
import com.sce.generated.test207.Test207State
import com.sce.generated.test207.Test207StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: The SCXML Processor MUST NOT allow cancel to affect events that were not raised in the same session.
@DisplayName("Test 207 -- W3C SCXML 6.3")
class Test207 : W3CTestBase<Test207State, Test207Event>() {
    override fun createStateMachine() = Test207StateMachine()
    override val expectedPassState: Test207State = Test207State.Pass
    override val timeoutMs: Long = 5000L
}
