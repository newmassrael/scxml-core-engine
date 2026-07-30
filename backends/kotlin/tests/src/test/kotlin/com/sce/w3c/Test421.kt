// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
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
