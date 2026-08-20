// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test567.scxml:1
package com.sce.w3c

import com.sce.generated.test567.Test567Event
import com.sce.generated.test567.Test567State
import com.sce.generated.test567.Test567StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: The processor MUST use any message content other than '_scxmleventname' to populate _event.data.
@DisplayName("Test 567 -- W3C SCXML C.2")
class Test567 : W3CHttpTestBase<Test567State, Test567Event>() {
    override fun createStateMachine() = Test567StateMachine(createEngine())
    override val expectedPassState: Test567State = Test567State.Pass
}
