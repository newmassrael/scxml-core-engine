// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 401d2ad22cf222caeef0633edc48b3c3fd2090ab46bb9a2c354f5be833096227
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test201.scxml:1
package com.sce.w3c

import com.sce.generated.test201.Test201Event
import com.sce.generated.test201.Test201State
import com.sce.generated.test201.Test201StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: Processors that support HTTP POST must use the value http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor for the "type" attribute
@DisplayName("Test 201 -- W3C SCXML 6.2")
class Test201 : W3CHttpTestBase<Test201State, Test201Event>() {
    override fun createStateMachine() = Test201StateMachine(createEngine())
    override val expectedPassState: Test201State = Test201State.Pass
}
