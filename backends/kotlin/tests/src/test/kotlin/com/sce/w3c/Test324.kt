// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test324.scxml:1
package com.sce.w3c

import com.sce.generated.test324.Test324Event
import com.sce.generated.test324.Test324State
import com.sce.generated.test324.Test324StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _name variable bound to the value of the 'name' attribute of the scxml element until the session terminates.
@DisplayName("Test 324 -- W3C SCXML 5.10")
class Test324 : W3CTestBase<Test324State, Test324Event>() {
    override fun createStateMachine() = Test324StateMachine(createEngine())
    override val expectedPassState: Test324State = Test324State.Pass
}
