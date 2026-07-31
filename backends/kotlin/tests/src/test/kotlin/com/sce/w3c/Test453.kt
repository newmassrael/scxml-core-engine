// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test453.scxml:1
package com.sce.w3c

import com.sce.generated.test453.Test453Event
import com.sce.generated.test453.Test453State
import com.sce.generated.test453.Test453StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel,  the SCXML Processor must accept any ECMAScript expression as a value expression.
@DisplayName("Test 453 -- W3C SCXML B.2")
class Test453 : W3CTestBase<Test453State, Test453Event>() {
    override fun createStateMachine() = Test453StateMachine(createEngine())
    override val expectedPassState: Test453State = Test453State.Pass
}
