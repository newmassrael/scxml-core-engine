// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f21fa6fe20b06255f5ff03ff01c6dbc9228fed62e399d58a912b19b086193a03
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test580.scxml:1
package com.sce.w3c

import com.sce.generated.test580.Test580Event
import com.sce.generated.test580.Test580State
import com.sce.generated.test580.Test580StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: It follows from the semantics of history states that they never end up in the state configuration
@DisplayName("Test 580 -- W3C SCXML 3.10")
class Test580 : W3CTestBase<Test580State, Test580Event>() {
    override fun createStateMachine() = Test580StateMachine(createEngine())
    override val expectedPassState: Test580State = Test580State.Pass
    override val timeoutMs: Long = 5000L
}
