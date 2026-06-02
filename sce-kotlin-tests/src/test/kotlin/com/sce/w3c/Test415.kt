// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
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
