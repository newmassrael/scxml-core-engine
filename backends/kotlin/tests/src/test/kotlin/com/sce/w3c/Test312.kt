// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test312.scxml:1
package com.sce.w3c

import com.sce.generated.test312.Test312Event
import com.sce.generated.test312.Test312State
import com.sce.generated.test312.Test312StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a value expression does not return a legal data value, the SCXML processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 312 -- W3C SCXML 5.9")
class Test312 : W3CTestBase<Test312State, Test312Event>() {
    override fun createStateMachine() = Test312StateMachine(createEngine())
    override val expectedPassState: Test312State = Test312State.Pass
}
