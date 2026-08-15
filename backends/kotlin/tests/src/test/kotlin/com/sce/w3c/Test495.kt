// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b5bef7d045160440c6e2790d4f2e0be757d7c1cc42dee75b2002b23fd477161e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test495.scxml:1
package com.sce.w3c

import com.sce.generated.test495.Test495Event
import com.sce.generated.test495.Test495State
import com.sce.generated.test495.Test495StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: If no errors occur, the receiving Processor MUST convert the message into an SCXML event, using the mapping defined above and insert it into the appropriate queue, as defined in Send Targets.
@DisplayName("Test 495 -- W3C SCXML C.1")
class Test495 : W3CTestBase<Test495State, Test495Event>() {
    override fun createStateMachine() = Test495StateMachine()
    override val expectedPassState: Test495State = Test495State.Pass
}
