// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test388.scxml:1
package com.sce.w3c

import com.sce.generated.test388.Test388Event
import com.sce.generated.test388.Test388State
import com.sce.generated.test388.Test388StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: After the parent state has been visited for the first time, if a transition is executed that takes the history state as its target, the SCXML processor MUST behave as if the transition had taken the stored state configuration as its target.
@DisplayName("Test 388 -- W3C SCXML 3.10")
class Test388 : W3CTestBase<Test388State, Test388Event>() {
    override fun createStateMachine() = Test388StateMachine(createEngine())
    override val expectedPassState: Test388State = Test388State.Pass
}
