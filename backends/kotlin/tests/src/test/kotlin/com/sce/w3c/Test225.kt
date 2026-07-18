// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388945
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test225.scxml:1
package com.sce.w3c

import com.sce.generated.test225.Test225Event
import com.sce.generated.test225.Test225State
import com.sce.generated.test225.Test225StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: n the automatically generated invoke identifier, platformid MUST be unique within the current session
@DisplayName("Test 225 -- W3C SCXML 6.4")
class Test225 : W3CTestBase<Test225State, Test225Event>() {
    override fun createStateMachine() = Test225StateMachine(createEngine())
    override val expectedPassState: Test225State = Test225State.Pass
}
