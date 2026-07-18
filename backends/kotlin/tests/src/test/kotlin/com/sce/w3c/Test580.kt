// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388945
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
