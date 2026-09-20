// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7b98e70bedf81ba26d5ed411dff10dd848488064e2842a1bb6ffe1d783a88c2a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test233.scxml:1
package com.sce.w3c

import com.sce.generated.test233.Test233Event
import com.sce.generated.test233.Test233State
import com.sce.generated.test233.Test233StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If there is a finalize handler in the instance of invoke that created the service that generated the event, the SCXML Processor MUST execute the code in that finalize handler right before it removes the event from the event queue for processing.
@DisplayName("Test 233 -- W3C SCXML 6.4")
class Test233 : W3CTestBase<Test233State, Test233Event>() {
    override fun createStateMachine() = Test233StateMachine(createEngine())
    override val expectedPassState: Test233State = Test233State.Pass
}
