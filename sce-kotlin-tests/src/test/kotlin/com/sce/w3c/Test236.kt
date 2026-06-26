// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d43b22670550c67cebe189489d0fdc39f585b0c09803917dea05e0ded254e31e
// generated-at: 1782434361
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test236.scxml:1
package com.sce.w3c

import com.sce.generated.test236.Test236Event
import com.sce.generated.test236.Test236State
import com.sce.generated.test236.Test236StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: The external service MUST NOT generate any other events after the invoke.done.invokeid event.
@DisplayName("Test 236 -- W3C SCXML 6.4")
class Test236 : W3CTestBase<Test236State, Test236Event>() {
    override fun createStateMachine() = Test236StateMachine()
    override val expectedPassState: Test236State = Test236State.Pass
    override val timeoutMs: Long = 5000L
}
