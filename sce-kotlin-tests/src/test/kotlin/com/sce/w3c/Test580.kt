// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test580.scxml:1
package com.sce.w3c

import com.sce.generated.test580.Test580Event
import com.sce.generated.test580.Test580State
import com.sce.generated.test580.Test580StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: It follows from the semantics of history states that they never end up in the state configuration
@DisplayName("Test 580 -- W3C SCXML 3.10")
class Test580 : W3CTestBase<Test580State, Test580Event>() {
    override fun createStateMachine() = Test580StateMachine(createEngine())
    override val expectedPassState: Test580State = Test580State.Pass
    override val timeoutMs: Long = 5000L
}
