// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test529.scxml:1
package com.sce.w3c

import com.sce.generated.test529.Test529Event
import com.sce.generated.test529.Test529State
import com.sce.generated.test529.Test529StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: If the 'expr' attribute is not present, the Processor MUST use the children of content as the output.
@DisplayName("Test 529 -- W3C SCXML 5.6")
class Test529 : W3CTestBase<Test529State, Test529Event>() {
    override fun createStateMachine() = Test529StateMachine(createEngine())
    override val expectedPassState: Test529State = Test529State.Pass
}
