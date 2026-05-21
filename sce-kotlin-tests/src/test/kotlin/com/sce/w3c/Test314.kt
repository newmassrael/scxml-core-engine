// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test314.scxml:1
package com.sce.w3c

import com.sce.generated.test314.Test314Event
import com.sce.generated.test314.Test314State
import com.sce.generated.test314.Test314StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If the SCXML processor waits until it evaluates the expressions at runtime to raise errors, it MUST raise errors caused by expressions returning illegal values at the points at which Appendix A Algorithm for SCXML Interpretation indicates that the expressions are to be evaluated.
@DisplayName("Test 314 -- W3C SCXML 5.9")
class Test314 : W3CTestBase<Test314State, Test314Event>() {
    override fun createStateMachine() = Test314StateMachine(createEngine())
    override val expectedPassState: Test314State = Test314State.Pass
}
