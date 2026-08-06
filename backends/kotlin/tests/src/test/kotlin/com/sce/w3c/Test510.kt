// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test510.scxml:1
package com.sce.w3c

import com.sce.generated.test510.Test510Event
import com.sce.generated.test510.Test510State
import com.sce.generated.test510.Test510StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: The SCXML Processor MUST validate the message it receives [via the Basic HTTP Event I/O Processor] and then MUST build the appropriate SCXML event and MUST add it to the external event queue
@DisplayName("Test 510 -- W3C SCXML C.2")
class Test510 : W3CHttpTestBase<Test510State, Test510Event>() {
    override fun createStateMachine() = Test510StateMachine(createEngine())
    override val expectedPassState: Test510State = Test510State.Pass
}
