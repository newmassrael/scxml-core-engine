// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a184cafe7a67a901b89b33f0188251221f7a1b0a549681a02c5ff67b5487c305
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test237.scxml:1
package com.sce.w3c

import com.sce.generated.test237.Test237Event
import com.sce.generated.test237.Test237State
import com.sce.generated.test237.Test237StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoking session takes a transition out of the state containing the invoke before it receives the 'done.invoke.id' event, the SCXML Processor MUST automatically cancel the invoked component and stop its processing.
@DisplayName("Test 237 -- W3C SCXML 6.4")
class Test237 : W3CTestBase<Test237State, Test237Event>() {
    override fun createStateMachine() = Test237StateMachine()
    override val expectedPassState: Test237State = Test237State.Pass
    override val timeoutMs: Long = 5000L
}
