// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 82c2acf711bf144ff090617437c4b567e5f475de105d2b1d2386e3cb7f2a1451
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test506.scxml:1
package com.sce.w3c

import com.sce.generated.test506.Test506Event
import com.sce.generated.test506.Test506State
import com.sce.generated.test506.Test506StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If a transition has 'type' of "internal", but its source state is not a compound state or its target states are not all proper descendents of its source state, its exit set is defined as if it had 'type' of "external".
@DisplayName("Test 506 -- W3C SCXML 3.13")
class Test506 : W3CTestBase<Test506State, Test506Event>() {
    override fun createStateMachine() = Test506StateMachine(createEngine())
    override val expectedPassState: Test506State = Test506State.Pass
}
