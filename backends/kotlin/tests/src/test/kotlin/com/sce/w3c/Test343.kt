// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test343.scxml:1
package com.sce.w3c

import com.sce.generated.test343.Test343Event
import com.sce.generated.test343.Test343State
import com.sce.generated.test343.Test343StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: If the 'location' attribute on a param element does not refer to a valid location in the data model, or if the evaluation of the 'expr' produces an error, the processor MUST ignore the name and value.
@DisplayName("Test 343 -- W3C SCXML 5.7")
class Test343 : W3CTestBase<Test343State, Test343Event>() {
    override fun createStateMachine() = Test343StateMachine(createEngine())
    override val expectedPassState: Test343State = Test343State.Pass
}
