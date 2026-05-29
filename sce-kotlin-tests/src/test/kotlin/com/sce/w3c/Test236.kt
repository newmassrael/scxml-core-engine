// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5e63a3ecc19b397697c3e24d727bc3c78cb748941f07d7f7c9d76cdea58d15a4
// generated-at: 1780032748
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
