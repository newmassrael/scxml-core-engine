// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a184cafe7a67a901b89b33f0188251221f7a1b0a549681a02c5ff67b5487c305
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test242.scxml:1
package com.sce.w3c

import com.sce.generated.test242.Test242Event
import com.sce.generated.test242.Test242State
import com.sce.generated.test242.Test242StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST also treat values specified by 'src' and content identically.
@DisplayName("Test 242 -- W3C SCXML 6.4")
class Test242 : W3CTestBase<Test242State, Test242Event>() {
    override fun createStateMachine() = Test242StateMachine()
    override val expectedPassState: Test242State = Test242State.Pass
    override val timeoutMs: Long = 5000L
}
