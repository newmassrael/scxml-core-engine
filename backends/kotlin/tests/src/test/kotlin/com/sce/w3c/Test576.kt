// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5acba0e3347282f793223e6756c0e705a2e09e70e21550d5eb5dc6ae9d6f33ae
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
package com.sce.w3c

import com.sce.generated.test576.Test576Event
import com.sce.generated.test576.Test576State
import com.sce.generated.test576.Test576StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, the SCXML Processor MUST enter the states specified by the 'initial' attribute, if it is present.
@DisplayName("Test 576 -- W3C SCXML 3.2")
class Test576 : W3CTestBase<Test576State, Test576Event>() {
    override fun createStateMachine() = Test576StateMachine()
    override val expectedPassState: Test576State = Test576State.Pass
}
