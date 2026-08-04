// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c9e08658681ef21dd3bd5428d9da1979a690ea0bbf7340f9b10920cbe666e5c5
// generated-at: 0
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
