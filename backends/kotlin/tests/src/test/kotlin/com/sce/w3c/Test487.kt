// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a184cafe7a67a901b89b33f0188251221f7a1b0a549681a02c5ff67b5487c305
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test487.scxml:1
package com.sce.w3c

import com.sce.generated.test487.Test487Event
import com.sce.generated.test487.Test487State
import com.sce.generated.test487.Test487StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the value specified (by 'expr' or children) is not a legal value for the location specified, the processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 487 -- W3C SCXML 5.4")
class Test487 : W3CTestBase<Test487State, Test487Event>() {
    override fun createStateMachine() = Test487StateMachine(createEngine())
    override val expectedPassState: Test487State = Test487State.Pass
}
