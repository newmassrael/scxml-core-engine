// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388945
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test230.scxml:1
package com.sce.w3c

import com.sce.generated.test230.Test230Event
import com.sce.generated.test230.Test230State
import com.sce.generated.test230.Test230StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the SCXML Processor autoforwards an event to the invoked process, all the fields specified in 5.11.1 The Internal Structure of Events MUST have the same values in the forwarded copy of the event
@DisplayName("Test 230 -- W3C SCXML 6.4")
class Test230 : W3CTestBase<Test230State, Test230Event>() {
    override fun createStateMachine() = Test230StateMachine(createEngine())
    override val expectedPassState: Test230State = Test230State.Final
    override val timeoutMs: Long = 5000L
}
