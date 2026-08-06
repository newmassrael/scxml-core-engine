// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test349.scxml:1
package com.sce.w3c

import com.sce.generated.test349.Test349Event
import com.sce.generated.test349.Test349State
import com.sce.generated.test349.Test349StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: source'. The sending SCXML Processor MUST populate this attribute with a URI that the receiving processor can use to reply to the sending processor. The receiving SCXML Processor MUST use this URI as the value of the 'origin' field in the event that it generates.
@DisplayName("Test 349 -- W3C SCXML C.1")
class Test349 : W3CTestBase<Test349State, Test349Event>() {
    override fun createStateMachine() = Test349StateMachine(createEngine())
    override val expectedPassState: Test349State = Test349State.Pass
}
