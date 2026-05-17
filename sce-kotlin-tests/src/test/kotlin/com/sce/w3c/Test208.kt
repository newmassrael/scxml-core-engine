// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test208.scxml:1
package com.sce.w3c

import com.sce.generated.test208.Test208Event
import com.sce.generated.test208.Test208State
import com.sce.generated.test208.Test208StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: The Processor SHOULD make its best attempt to cancel all delayed events with the specified id.
@DisplayName("Test 208 -- W3C SCXML 6.3")
class Test208 : W3CTestBase<Test208State, Test208Event>() {
    override fun createStateMachine() = Test208StateMachine()
    override val expectedPassState: Test208State = Test208State.Pass
    override val timeoutMs: Long = 5000L
}
