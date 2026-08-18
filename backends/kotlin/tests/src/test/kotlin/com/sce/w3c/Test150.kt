// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test150.scxml:1
package com.sce.w3c

import com.sce.generated.test150.Test150Event
import com.sce.generated.test150.Test150State
import com.sce.generated.test150.Test150StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, the SCXML processor MUST declare a new variable if the one specified by 'item' is not already defined.
@DisplayName("Test 150 -- W3C SCXML 4.6")
class Test150 : W3CTestBase<Test150State, Test150Event>() {
    override fun createStateMachine() = Test150StateMachine(createEngine())
    override val expectedPassState: Test150State = Test150State.Pass
}
