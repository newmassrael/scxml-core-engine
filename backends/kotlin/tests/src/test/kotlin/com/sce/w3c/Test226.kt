// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test226.scxml:1
package com.sce.w3c

import com.sce.generated.test226.Test226Event
import com.sce.generated.test226.Test226State
import com.sce.generated.test226.Test226StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the invoke element is executed, the SCXML Processor MUST start a new logical instance of the external service specified in 'type' or 'typexpr', passing it the URL specified by 'src' or the data specified by content, or param.
@DisplayName("Test 226 -- W3C SCXML 6.4")
class Test226 : W3CTestBase<Test226State, Test226Event>() {
    override fun createStateMachine() = Test226StateMachine(createEngine())
    override val expectedPassState: Test226State = Test226State.Pass
}
