// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388945
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
