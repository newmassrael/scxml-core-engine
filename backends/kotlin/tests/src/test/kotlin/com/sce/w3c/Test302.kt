// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f21fa6fe20b06255f5ff03ff01c6dbc9228fed62e399d58a912b19b086193a03
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test302.scxml:1
package com.sce.w3c

import com.sce.generated.test302.Test302Event
import com.sce.generated.test302.Test302State
import com.sce.generated.test302.Test302StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: The SCXML Processor MUST evaluate any script element that is a child of scxml at document load time. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 302 -- W3C SCXML 5.8")
class Test302 : W3CTestBase<Test302State, Test302Event>() {
    override fun createStateMachine() = Test302StateMachine(createEngine())
    override val expectedPassState: Test302State = Test302State.Pass
}
