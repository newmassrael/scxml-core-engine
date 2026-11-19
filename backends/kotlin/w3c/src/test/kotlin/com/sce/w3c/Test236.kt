// SCE-GENERATED — DO NOT EDIT
// source-hash: 5a095d22970297c11305d0511d6a4144b57baee226525a5af9b1a3c5e44b7d3e
// source: test236.scxml
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
