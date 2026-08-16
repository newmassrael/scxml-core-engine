// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f21fa6fe20b06255f5ff03ff01c6dbc9228fed62e399d58a912b19b086193a03
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test459.scxml:1
package com.sce.w3c

import com.sce.generated.test459.Test459Event
import com.sce.generated.test459.Test459State
import com.sce.generated.test459.Test459StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the iteration order for the foreach element is the order of the underlying ECMAScript array, and goes from an index of 0 by increments of one to an index of array_name.length - 1.
@DisplayName("Test 459 -- W3C SCXML B.2")
class Test459 : W3CTestBase<Test459State, Test459Event>() {
    override fun createStateMachine() = Test459StateMachine(createEngine())
    override val expectedPassState: Test459State = Test459State.Pass
}
