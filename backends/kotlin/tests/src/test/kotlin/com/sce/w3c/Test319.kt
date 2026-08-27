// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 42298195b20865d87e273e6a89fd9b7e20af26d02f54273007f21322d047b5d4
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test319.scxml:1
package com.sce.w3c

import com.sce.generated.test319.Test319Event
import com.sce.generated.test319.Test319State
import com.sce.generated.test319.Test319StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST NOT bind _event at initialization time until the first event is processed.
@DisplayName("Test 319 -- W3C SCXML 5.10")
class Test319 : W3CTestBase<Test319State, Test319Event>() {
    override fun createStateMachine() = Test319StateMachine(createEngine())
    override val expectedPassState: Test319State = Test319State.Pass
}
