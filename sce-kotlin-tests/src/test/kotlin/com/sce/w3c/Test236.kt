// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
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
