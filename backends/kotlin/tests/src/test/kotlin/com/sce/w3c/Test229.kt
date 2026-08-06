// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test229.scxml:1
package com.sce.w3c

import com.sce.generated.test229.Test229Event
import com.sce.generated.test229.Test229State
import com.sce.generated.test229.Test229StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the 'autoforward' attribute is set to true, the SCXML Processor MUST send an exact copy of every external event it receives to the invoked process.
@DisplayName("Test 229 -- W3C SCXML 6.4")
class Test229 : W3CTestBase<Test229State, Test229Event>() {
    override fun createStateMachine() = Test229StateMachine()
    override val expectedPassState: Test229State = Test229State.Pass
    override val timeoutMs: Long = 5000L
}
