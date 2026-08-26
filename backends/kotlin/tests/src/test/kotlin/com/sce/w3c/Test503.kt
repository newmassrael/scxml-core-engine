// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test503.scxml:1
package com.sce.w3c

import com.sce.generated.test503.Test503Event
import com.sce.generated.test503.Test503State
import com.sce.generated.test503.Test503StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the transition does not contain a 'target', its exit set is empty.
@DisplayName("Test 503 -- W3C SCXML 3.13")
class Test503 : W3CTestBase<Test503State, Test503Event>() {
    override fun createStateMachine() = Test503StateMachine(createEngine())
    override val expectedPassState: Test503State = Test503State.Pass
}
