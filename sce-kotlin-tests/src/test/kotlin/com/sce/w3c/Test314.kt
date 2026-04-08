// GENERATED -- DO NOT EDIT (sce-codegen)
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
