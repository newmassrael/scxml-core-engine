// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 50d6eb36f321e50c2a6e5457f0a900b925f832ee57619f9b6a33cf22bd75d4e1
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test500.scxml:1
package com.sce.w3c

import com.sce.generated.test500.Test500Event
import com.sce.generated.test500.Test500State
import com.sce.generated.test500.Test500StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: SCXML Processors that support the BasicHTTP Event I/O Processor MUST maintain a 'scxml' entry in the _ioprocessors system variable. The Processor MUST maintain a 'location' field inside this entry whose value holds an address that external entities can use to communicate with this SCXML session using the SCXML Event I/O Processor.
@DisplayName("Test 500 -- W3C SCXML C.1")
class Test500 : W3CTestBase<Test500State, Test500Event>() {
    override fun createStateMachine() = Test500StateMachine(createEngine())
    override val expectedPassState: Test500State = Test500State.Pass
}
