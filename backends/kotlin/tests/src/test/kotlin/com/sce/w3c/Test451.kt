// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test451.scxml:1
package com.sce.w3c

import com.sce.generated.test451.Test451Event
import com.sce.generated.test451.Test451State
import com.sce.generated.test451.Test451StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must add an ECMAScript
@DisplayName("Test 451 -- W3C SCXML B.2")
class Test451 : W3CTestBase<Test451State, Test451Event>() {
    override fun createStateMachine() = Test451StateMachine()
    override val expectedPassState: Test451State = Test451State.Pass
}
