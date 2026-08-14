// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test199.scxml:1
package com.sce.w3c

import com.sce.generated.test199.Test199Event
import com.sce.generated.test199.Test199State
import com.sce.generated.test199.Test199StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML Processor does not support the type that is specified, it MUST place the event error.execution on the internal event queue.
@DisplayName("Test 199 -- W3C SCXML 6.2")
class Test199 : W3CTestBase<Test199State, Test199Event>() {
    override fun createStateMachine() = Test199StateMachine()
    override val expectedPassState: Test199State = Test199State.Pass
}
