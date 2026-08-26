// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
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
