// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test376.scxml:1
package com.sce.w3c

import com.sce.generated.test376.Test376Event
import com.sce.generated.test376.Test376State
import com.sce.generated.test376.Test376StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.8: The SCXML processor MUST treat each [onentry] handler as a separate block of executable content.
@DisplayName("Test 376 -- W3C SCXML 3.8")
class Test376 : W3CTestBase<Test376State, Test376Event>() {
    override fun createStateMachine() = Test376StateMachine(createEngine())
    override val expectedPassState: Test376State = Test376State.Pass
}
