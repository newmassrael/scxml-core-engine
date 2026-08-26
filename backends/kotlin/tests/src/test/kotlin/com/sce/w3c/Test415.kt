// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test415.scxml:1
package com.sce.w3c

import com.sce.generated.test415.Test415Event
import com.sce.generated.test415.Test415State
import com.sce.generated.test415.Test415StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If it [the SCXML Processor] has entered a final state that is a child of scxml [during the last microstep], it MUST halt processing.
@DisplayName("Test 415 -- W3C SCXML 3.13")
class Test415 : W3CTestBase<Test415State, Test415Event>() {
    override fun createStateMachine() = Test415StateMachine()
    override val expectedPassState: Test415State = Test415State.Final
}
