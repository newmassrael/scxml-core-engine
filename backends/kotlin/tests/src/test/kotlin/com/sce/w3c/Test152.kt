// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 465642caa5c7ae5f006b7e4c3302ebaf26878f27c380322c3cf9d87ca35b0ee6
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test152.scxml:1
package com.sce.w3c

import com.sce.generated.test152.Test152Event
import com.sce.generated.test152.Test152State
import com.sce.generated.test152.Test152StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, if 'array' does not evaluate to a legal iterable collection, or if 'item' does not specify a legal variable name, the SCXML processor MUST terminate execution of the foreach element and the block that contains it, and place the error error.execution on the internal event queue.
@DisplayName("Test 152 -- W3C SCXML 4.6")
class Test152 : W3CTestBase<Test152State, Test152Event>() {
    override fun createStateMachine() = Test152StateMachine(createEngine())
    override val expectedPassState: Test152State = Test152State.Pass
}
