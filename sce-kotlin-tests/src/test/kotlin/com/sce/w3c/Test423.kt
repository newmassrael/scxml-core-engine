// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
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
