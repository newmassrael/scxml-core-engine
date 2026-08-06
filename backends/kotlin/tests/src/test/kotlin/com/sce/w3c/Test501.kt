// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 97155411577c11a793c509f8eca92ed763090b054ea00a1be8ebdfe84ee878d0
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test501.scxml:1
package com.sce.w3c

import com.sce.generated.test501.Test501Event
import com.sce.generated.test501.Test501State
import com.sce.generated.test501.Test501StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: The 'location' field inside the entry for the SCXML Event I/O Processor in the _ioprocessors system variable MUST hold an address that external entities can use to communicate with this SCXML session using the SCXML Event I/O Processor.
@DisplayName("Test 501 -- W3C SCXML C.1")
class Test501 : W3CTestBase<Test501State, Test501Event>() {
    override fun createStateMachine() = Test501StateMachine(createEngine())
    override val expectedPassState: Test501State = Test501State.Pass
}
