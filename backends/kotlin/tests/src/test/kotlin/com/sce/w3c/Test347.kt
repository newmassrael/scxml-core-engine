// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test347.scxml:1
package com.sce.w3c

import com.sce.generated.test347.Test347Event
import com.sce.generated.test347.Test347State
import com.sce.generated.test347.Test347StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: SCXML Processors MUST support sending messages to and receiving messages from other SCXML sessions using the SCXML Event I/O Processor.
@DisplayName("Test 347 -- W3C SCXML C.1")
class Test347 : W3CTestBase<Test347State, Test347Event>() {
    override fun createStateMachine() = Test347StateMachine()
    override val expectedPassState: Test347State = Test347State.Pass
}
