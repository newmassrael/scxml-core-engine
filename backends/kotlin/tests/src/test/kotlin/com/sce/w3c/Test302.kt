// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test302.scxml:1
package com.sce.w3c

import com.sce.generated.test302.Test302Event
import com.sce.generated.test302.Test302State
import com.sce.generated.test302.Test302StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: The SCXML Processor MUST evaluate any script element that is a child of scxml at document load time. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 302 -- W3C SCXML 5.8")
class Test302 : W3CTestBase<Test302State, Test302Event>() {
    override fun createStateMachine() = Test302StateMachine(createEngine())
    override val expectedPassState: Test302State = Test302State.Pass
}
