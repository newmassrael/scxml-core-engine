// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 88fb4239c7bf755fc1c6cd464b6fde12b78deadf029e10850a42d88cb83ecc77
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
