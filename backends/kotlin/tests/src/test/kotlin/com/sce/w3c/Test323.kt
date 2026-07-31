// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test323.scxml:1
package com.sce.w3c

import com.sce.generated.test323.Test323Event
import com.sce.generated.test323.Test323State
import com.sce.generated.test323.Test323StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _name at load time to the value of the 'name' attribute of the scxml element. 	a
@DisplayName("Test 323 -- W3C SCXML 5.10")
class Test323 : W3CTestBase<Test323State, Test323Event>() {
    override fun createStateMachine() = Test323StateMachine(createEngine())
    override val expectedPassState: Test323State = Test323State.Pass
}
