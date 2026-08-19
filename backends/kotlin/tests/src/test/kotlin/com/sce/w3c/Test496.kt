// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5b0237a7a83721c40de92b1914fb5f3ab69530a228f19b8f33cd3af4e27ebf24
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test496.scxml:1
package com.sce.w3c

import com.sce.generated.test496.Test496Event
import com.sce.generated.test496.Test496State
import com.sce.generated.test496.Test496StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: If the sending SCXML session specifies a session that does not exist or is inaccessible, the SCXML Processor MUST place the error error.communication on the internal event queue of the sending session.
@DisplayName("Test 496 -- W3C SCXML C.1")
class Test496 : W3CTestBase<Test496State, Test496Event>() {
    override fun createStateMachine() = Test496StateMachine(createEngine())
    override val expectedPassState: Test496State = Test496State.Pass
}
