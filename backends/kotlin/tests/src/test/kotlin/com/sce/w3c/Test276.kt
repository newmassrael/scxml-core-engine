// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 15d6029f4f85b34e5f54293320d31d5c234aec7e25e520553a3723febef59859
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test276.scxml:1
package com.sce.w3c

import com.sce.generated.test276.Test276Event
import com.sce.generated.test276.Test276State
import com.sce.generated.test276.Test276StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: The SCXML Processor MUST allow the environment to provide values for top-level data elements at instantiation time. (Top-level data elements are those that are children of the datamodel element that is a child of scxml). Specifically, the Processor MUST use the values provided at instantiation time instead of those contained in these data elements.
@DisplayName("Test 276 -- W3C SCXML 5.3")
class Test276 : W3CTestBase<Test276State, Test276Event>() {
    override fun createStateMachine() = Test276StateMachine(createEngine())
    override val expectedPassState: Test276State = Test276State.Pass
    override val timeoutMs: Long = 5000L
}
