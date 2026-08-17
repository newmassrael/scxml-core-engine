// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 039ad389d30ffb729c7c2441931b41f36924cbf4b6013115d42ef3094467532b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test415.scxml:1
package com.sce.w3c

import com.sce.generated.test415.Test415Event
import com.sce.generated.test415.Test415State
import com.sce.generated.test415.Test415StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If it [the SCXML Processor] has entered a final state that is a child of scxml [during the last microstep], it MUST halt processing.
@DisplayName("Test 415 -- W3C SCXML 3.13")
class Test415 : W3CTestBase<Test415State, Test415Event>() {
    override fun createStateMachine() = Test415StateMachine()
    override val expectedPassState: Test415State = Test415State.Final
}
