// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test223.scxml:1
package com.sce.w3c

import com.sce.generated.test223.Test223Event
import com.sce.generated.test223.Test223State
import com.sce.generated.test223.Test223StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the 'idlocation' attribute is present, the SCXML Processor MUST generate an id automatically when the invoke element is evaluated and store it in the location specified by 'idlocation'.
@DisplayName("Test 223 -- W3C SCXML 6.4")
class Test223 : W3CTestBase<Test223State, Test223Event>() {
    override fun createStateMachine() = Test223StateMachine(createEngine())
    override val expectedPassState: Test223State = Test223State.Pass
}
