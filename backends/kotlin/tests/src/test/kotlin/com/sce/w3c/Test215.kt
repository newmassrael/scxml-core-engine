// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test215.scxml:1
package com.sce.w3c

import com.sce.generated.test215.Test215Event
import com.sce.generated.test215.Test215State
import com.sce.generated.test215.Test215StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the typeexpr attribute is present, the SCXML Processor MUST evaluate it when the parent invoke element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 215 -- W3C SCXML 6.4")
class Test215 : W3CTestBase<Test215State, Test215Event>() {
    override fun createStateMachine() = Test215StateMachine(createEngine())
    override val expectedPassState: Test215State = Test215State.Pass
}
