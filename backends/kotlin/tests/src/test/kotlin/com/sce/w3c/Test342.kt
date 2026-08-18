// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test342.scxml:1
package com.sce.w3c

import com.sce.generated.test342.Test342Event
import com.sce.generated.test342.Test342State
import com.sce.generated.test342.Test342StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST set the name field (of the _event variable) to the name of the event.
@DisplayName("Test 342 -- W3C SCXML 5.10")
class Test342 : W3CTestBase<Test342State, Test342Event>() {
    override fun createStateMachine() = Test342StateMachine(createEngine())
    override val expectedPassState: Test342State = Test342State.Pass
}
