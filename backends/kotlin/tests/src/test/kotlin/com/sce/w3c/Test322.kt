// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388945
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test322.scxml:1
package com.sce.w3c

import com.sce.generated.test322.Test322Event
import com.sce.generated.test322.Test322State
import com.sce.generated.test322.Test322StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _sessionid variable bound to the system-generated id until the session terminates.
@DisplayName("Test 322 -- W3C SCXML 5.10")
class Test322 : W3CTestBase<Test322State, Test322Event>() {
    override fun createStateMachine() = Test322StateMachine(createEngine())
    override val expectedPassState: Test322State = Test322State.Pass
}
