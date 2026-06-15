// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 030a39123c8149accb30146fc4a4999b6e8826a330653d219a562116c552e0d8
// generated-at: 1781483328
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test423.scxml:1
package com.sce.w3c

import com.sce.generated.test423.Test423Event
import com.sce.generated.test423.Test423State
import com.sce.generated.test423.Test423StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Then [after invoking the new invoke handlers since the last macrostep] the Processor MUST remove events from the external event queue, waiting till events appear if necessary, until it finds one that enables a non-empty optimal transition set in the current configuration.
@DisplayName("Test 423 -- W3C SCXML 3.13")
class Test423 : W3CTestBase<Test423State, Test423Event>() {
    override fun createStateMachine() = Test423StateMachine()
    override val expectedPassState: Test423State = Test423State.Pass
    override val timeoutMs: Long = 5000L
}
