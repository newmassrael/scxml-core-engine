// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test352.scxml:1
package com.sce.w3c

import com.sce.generated.test352.Test352Event
import com.sce.generated.test352.Test352State
import com.sce.generated.test352.Test352StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: 'sourcetype'. The sending SCXML Processor MUST assign this attribute the value "scxml". (Note that other types of senders will assign different values.) The receiving Processor MUST use this value as the value of the 'origintype' field of the event that it generates.
@DisplayName("Test 352 -- W3C SCXML C.1")
class Test352 : W3CTestBase<Test352State, Test352Event>() {
    override fun createStateMachine() = Test352StateMachine(createEngine())
    override val expectedPassState: Test352State = Test352State.Pass
}
