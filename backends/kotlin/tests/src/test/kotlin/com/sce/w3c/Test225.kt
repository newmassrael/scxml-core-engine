// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 88fb4239c7bf755fc1c6cd464b6fde12b78deadf029e10850a42d88cb83ecc77
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test225.scxml:1
package com.sce.w3c

import com.sce.generated.test225.Test225Event
import com.sce.generated.test225.Test225State
import com.sce.generated.test225.Test225StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: n the automatically generated invoke identifier, platformid MUST be unique within the current session
@DisplayName("Test 225 -- W3C SCXML 6.4")
class Test225 : W3CTestBase<Test225State, Test225Event>() {
    override fun createStateMachine() = Test225StateMachine(createEngine())
    override val expectedPassState: Test225State = Test225State.Pass
}
