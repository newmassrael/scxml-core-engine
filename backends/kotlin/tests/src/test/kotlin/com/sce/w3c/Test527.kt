// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test527.scxml:1
package com.sce.w3c

import com.sce.generated.test527.Test527Event
import com.sce.generated.test527.Test527State
import com.sce.generated.test527.Test527StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: When the SCXML Processor evaluates the content element, if the 'expr' value expression is present, the Processor MUST evaluate it and use the result as the output of the content element.
@DisplayName("Test 527 -- W3C SCXML 5.6")
class Test527 : W3CTestBase<Test527State, Test527Event>() {
    override fun createStateMachine() = Test527StateMachine(createEngine())
    override val expectedPassState: Test527State = Test527State.Pass
}
