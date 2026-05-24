// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2e32d261d6350eb3a25f2f20128ae90019b36b8835127308d167f05b44688be3
// generated-at: 1779589482
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test421.scxml:1
package com.sce.w3c

import com.sce.generated.test421.Test421Event
import com.sce.generated.test421.Test421State
import com.sce.generated.test421.Test421StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the set (of eventless transitions) is empty, the Processor MUST remove events from the internal event queue until the queue is empty or it finds an event that enables a non-empty optimal transition set in the current configuration.
@DisplayName("Test 421 -- W3C SCXML 3.13")
class Test421 : W3CTestBase<Test421State, Test421Event>() {
    override fun createStateMachine() = Test421StateMachine()
    override val expectedPassState: Test421State = Test421State.Pass
}
