// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test344.scxml:1
package com.sce.w3c

import com.sce.generated.test344.Test344Event
import com.sce.generated.test344.Test344State
import com.sce.generated.test344.Test344StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST place the error 'error.execution' in the internal event queue.
@DisplayName("Test 344 -- W3C SCXML 5.9")
class Test344 : W3CTestBase<Test344State, Test344Event>() {
    override fun createStateMachine() = Test344StateMachine(createEngine())
    override val expectedPassState: Test344State = Test344State.Pass
}
