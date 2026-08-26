// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test348.scxml:1
package com.sce.w3c

import com.sce.generated.test348.Test348Event
import com.sce.generated.test348.Test348State
import com.sce.generated.test348.Test348StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: name'. The sending SCXML Processor MUST take the value of this attribute from the 'event' attribute of the send element. The receiving SCXML Processor MUST use it as the value the 'name' field in the event that it generates.
@DisplayName("Test 348 -- W3C SCXML C.1")
class Test348 : W3CTestBase<Test348State, Test348Event>() {
    override fun createStateMachine() = Test348StateMachine()
    override val expectedPassState: Test348State = Test348State.Pass
}
