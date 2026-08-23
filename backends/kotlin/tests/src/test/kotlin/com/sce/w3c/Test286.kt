// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1f4fc251a4bb4df71320b116cc055aa1687156c3a3402c346abf1bd3694d0437
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test286.scxml:1
package com.sce.w3c

import com.sce.generated.test286.Test286Event
import com.sce.generated.test286.Test286State
import com.sce.generated.test286.Test286StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the location expression of an assign does not denote a valid location in the datamodel the processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 286 -- W3C SCXML 5.4")
class Test286 : W3CTestBase<Test286State, Test286Event>() {
    override fun createStateMachine() = Test286StateMachine(createEngine())
    override val expectedPassState: Test286State = Test286State.Pass
}
